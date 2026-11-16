//! `adapdf fix <file.docx>`: lists the problems the tool can fix, asks about
//! each one, applies the chosen fixes, and shows the new results.
//!
//! The questions go to the error stream and the answers come from the input
//! stream, so a test can drive the command with a pipe. The result (the backup,
//! the fixes applied and the new check) goes to the output stream, and only
//! after the file is written. When the tool fails (exit code 2), nothing is
//! written to the output stream.
//!
//! Exit codes (spec 02): 0 = no errors remain, 1 = errors remain, 2 = the tool
//! failed.

use std::io::{BufRead, Write};
use std::path::Path;

use crate::findings::FixId;
use crate::render;

use super::{
    Availability, Environment, FixInput, FixOptions, FixRequest, Offer, describe, fix_file, offers,
    preflight,
};

/// How a yes/no answer is read.
enum Answer {
    Yes,
    No,
}

struct Prompter<'a> {
    input: &'a mut dyn BufRead,
    ask: &'a mut dyn Write,
}

impl Prompter<'_> {
    /// Writes the question and reads one line. `None` at the end of the input.
    fn line(&mut self, question: &str) -> Option<String> {
        let _ = write!(self.ask, "{question}");
        let _ = self.ask.flush();
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) | Err(_) => {
                let _ = writeln!(self.ask);
                None
            }
            Ok(_) => Some(line.trim().to_string()),
        }
    }

    /// Asks until the answer is `y`, `yes`, `n`, `no` or an empty line. The end
    /// of the input is "no".
    fn yes_no(&mut self, question: &str) -> Answer {
        loop {
            let Some(line) = self.line(&format!("{question} [y/n] ")) else {
                return Answer::No;
            };
            match line.to_lowercase().as_str() {
                "y" | "yes" => return Answer::Yes,
                "n" | "no" | "" => return Answer::No,
                _ => {
                    let _ = writeln!(self.ask, "Please answer y or n.");
                }
            }
        }
    }

    fn note(&mut self, text: &str) {
        let _ = writeln!(self.ask, "{text}");
    }
}

/// Asks about one offer. Returns the request to apply, if the user chose it.
fn ask_about(prompter: &mut Prompter, offer: &Offer, options: &FixOptions) -> Option<FixRequest> {
    let mut shown = String::new();
    render::write_finding(&mut shown, &offer.finding);
    prompter.note(shown.trim_end());
    match &offer.availability {
        Availability::AtExport => {
            prompter.note(
                "    Word does this when the PDF is exported. There is nothing to confirm now.",
            );
            return None;
        }
        Availability::Unavailable(reason) => {
            prompter.note(&format!("    The tool cannot fix this now: {reason}"));
            return None;
        }
        Availability::Ready => {}
    }
    prompter.note(&format!("    Fix: {}", describe(offer.fix, options)));
    match offer.fix {
        FixId::SetTitle => {
            let title = prompter.line("    Type the title (press Enter to skip): ")?;
            (!title.is_empty()).then(|| offer.request(FixInput::Text(title)))
        }
        FixId::SetAltText => {
            if let Answer::Yes =
                prompter.yes_no("    Mark it as decorative (it adds no information)?")
            {
                return Some(offer.request(FixInput::Decorative));
            }
            let alt = prompter.line("    Type the alt text (press Enter to skip): ")?;
            (!alt.is_empty()).then(|| offer.request(FixInput::Text(alt)))
        }
        FixId::MarkAnnotationsDecorative => {
            match prompter.yes_no(
                "    Does the alt text of the screenshot describe what the arrows and boxes show? Mark them decorative?",
            ) {
                Answer::Yes => Some(offer.request(FixInput::Confirm)),
                Answer::No => None,
            }
        }
        _ => match prompter.yes_no("    Apply this fix?") {
            Answer::Yes => Some(offer.request(FixInput::Confirm)),
            Answer::No => None,
        },
    }
}

/// Runs `adapdf fix` and returns the exit code.
pub fn run(
    path: &Path,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    env: &Environment,
) -> u8 {
    let options = FixOptions::default();
    let fail = |err: &mut dyn Write, message: String| {
        let _ = writeln!(err, "adapdf: {message}");
        2
    };
    // Stop at once when Word has the file open: nothing has been shown yet.
    if let Err(e) = preflight(env.fs, path) {
        return fail(err, e.to_string());
    }
    let bytes = match env.fs.read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return fail(
                err,
                format!("{}: the file cannot be read: {e}", path.display()),
            );
        }
    };
    let offers = match offers(&bytes, &options, env.fonts) {
        Ok(offers) => offers,
        Err(e) => return fail(err, format!("{}: {e}", path.display())),
    };

    let mut prompter = Prompter { input, ask: err };
    prompter.note(&format!(
        "Checking {}: {} problem(s) the tool can fix.",
        path.display(),
        offers.len()
    ));
    let requests: Vec<FixRequest> = offers
        .iter()
        .filter_map(|offer| ask_about(&mut prompter, offer, &options))
        .collect();

    let outcome = match fix_file(path, &requests, &options, env) {
        Ok(outcome) => outcome,
        Err(e) => return fail(err, e.to_string()),
    };

    let name = path.display().to_string();
    let mut report = String::new();
    match &outcome.backup {
        Some(backup) => report.push_str(&format!(
            "Backup: {}\n",
            backup.file_name().map_or_else(
                || backup.display().to_string(),
                |n| n.to_string_lossy().into_owned()
            )
        )),
        None => report.push_str("No fixes were written. No backup was made.\n"),
    }
    if !outcome.applied.is_empty() {
        report.push_str(&format!("Fixes applied ({}):\n", outcome.applied.len()));
        for applied in &outcome.applied {
            report.push_str(&format!(
                "  [{}] {}\n",
                applied.finding.rule_id, applied.summary
            ));
        }
    }
    if !outcome.at_export.is_empty() {
        report.push_str(&format!(
            "Done when the PDF is exported ({}):\n",
            outcome.at_export.len()
        ));
        for finding in &outcome.at_export {
            report.push_str(&format!("  [{}] {}\n", finding.rule_id, finding.message));
        }
    }
    if !outcome.refused.is_empty() {
        report.push_str(&format!("Not applied ({}):\n", outcome.refused.len()));
        for refusal in &outcome.refused {
            report.push_str(&format!(
                "  [{}] {}\n",
                refusal.finding.rule_id, refusal.reason
            ));
        }
    }
    report.push('\n');
    report.push_str(&render::to_text(&name, &outcome.findings));
    let _ = out.write_all(report.as_bytes());
    if render::has_errors(&outcome.findings) {
        1
    } else {
        0
    }
}
