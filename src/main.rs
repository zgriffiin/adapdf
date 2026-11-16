//! The `adapdf` command line.
//!
//! Exit codes: 0 = no errors (warnings are allowed), 1 = errors found,
//! 2 = the tool failed (for example the file cannot be read).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use adapdf::fonts::SystemFonts;
use adapdf::render;

#[derive(Parser)]
#[command(
    version,
    about = "Checks Word training guides for accessibility problems."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a .docx file and list the problems found.
    Check {
        /// The .docx file to check.
        file: PathBuf,
        /// Print the result as JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Fix problems in a .docx file. Asks before each fix, and writes a
    /// backup copy first.
    Fix {
        /// The .docx file to fix.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { file, json } => check(&file, json),
        Command::Fix { file } => fix(&file),
    }
}

fn fix(file: &std::path::Path) -> ExitCode {
    let fonts = SystemFonts::from_environment();
    let env = adapdf::fix::Environment {
        fs: &adapdf::fix::RealFs,
        now: adapdf::fix::Timestamp::now_utc(),
        fonts: &fonts,
    };
    let stdin = std::io::stdin();
    let code = adapdf::fix::interactive::run(
        file,
        &mut stdin.lock(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
        &env,
    );
    ExitCode::from(code)
}

fn check(file: &std::path::Path, json: bool) -> ExitCode {
    let findings = match adapdf::check_file(file, &SystemFonts::from_environment()) {
        Ok(findings) => findings,
        Err(error) => {
            eprintln!("adapdf: {}: {error}", file.display());
            return ExitCode::from(2);
        }
    };
    let name = file.display().to_string();
    if json {
        println!("{}", render::to_json(&name, &findings));
    } else {
        print!("{}", render::to_text(&name, &findings));
    }
    if render::has_errors(&findings) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
