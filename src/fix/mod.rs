//! Stage 2 of the pipeline (spec 02): fix what the pre-check found.
//!
//! The fixes are byte-range splices on the original XML text (see [`xml`]).
//! A part is never written again from a tree. The new .docx copies every
//! entry that did not change as it is, and replaces only the changed parts.
//!
//! The pure part is [`apply`]: bytes of a .docx and a list of chosen fixes in,
//! bytes of the fixed .docx out. [`fix_file`] adds the safety rules around it:
//! Word must not have the file open, the backup is written first, and the new
//! file is written to a temporary file and moved over the original. [`offers`]
//! lists what the tool can fix. [`interactive`] is the `adapdf fix` command.
//!
//! Fixes by fix ID:
//!
//! - Auto: `SetDefaultLanguage`, `ApplyHeadingStyle`, `DeleteEmptyHeading`,
//!   `ApplyListStyle`, `MakeImageInline`, `DeleteEmptyParagraphs`,
//!   `MarkHeaderRow`, `DeleteEmptyRow`, `DisallowRowBreak`, `AlignLeft`.
//!   `UpdateFields` writes nothing: Word updates the fields at export (M4).
//! - Ask: `SetTitle` and `SetAltText` take the text the user typed.
//!   `MarkAnnotationsDecorative` takes the user's confirmation. The tool never
//!   makes up text, and it rejects text that is empty or only spaces.
//!
//! A fix that the tool cannot do safely is refused with a message, and the file
//! is not changed for it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use roxmltree::Node;

use crate::docx::{CORE_PART, DOCUMENT_PART, Package, STYLES_PART, WordNode, parse_xml};
use crate::findings::{Finding, FixId, FixKind, RuleId};
use crate::fonts::FontLookup;
use crate::model::Document;
use crate::rules;

mod drawings;
pub mod interactive;
#[cfg(windows)]
#[allow(unsafe_code)]
mod local_time;
mod metadata;
mod package;
mod paragraphs;
mod safety;
mod tables;
pub mod xml;

pub use safety::{FileSystem, RealFs, Timestamp, find_lock_file, lock_file_names, open_for_fixing};
pub use xml::Splice;

use paragraphs::ParagraphEdit;
use tables::RowEdit;
use xml::Splices;

/// The reasons a fix run stops. The file is not changed.
#[derive(Debug, thiserror::Error)]
pub enum FixError {
    #[error(
        "{} is open in Word (the lock file {} is next to it). Close the document in Word, then run the fix again.",
        file.display(),
        lock.display()
    )]
    LockFile { file: PathBuf, lock: PathBuf },

    #[error(
        "{} cannot be opened for writing ({source}). Another program may have it open, or it may be read-only. Close it or change it, then run the fix again.",
        file.display()
    )]
    CannotWrite {
        file: PathBuf,
        source: std::io::Error,
    },

    #[error(
        "the backup {} cannot be written ({source}). Nothing was changed.",
        backup.display()
    )]
    Backup {
        backup: PathBuf,
        source: std::io::Error,
    },

    #[error(
        "the fixed file cannot be written ({source}). {} is unchanged.",
        file.display()
    )]
    Write {
        file: PathBuf,
        source: std::io::Error,
    },

    #[error(transparent)]
    Document(#[from] crate::error::Error),

    #[error(
        "two of the chosen fixes change the same text, so none of them was applied. Apply one of them, then run the fix again"
    )]
    Conflict,

    #[error(
        "the fixed document did not pass the tool's own check ({0}). Nothing was written. This is a fault in the tool"
    )]
    Invalid(String),
}

/// What the user gives for a fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixInput {
    /// The user said yes. For an auto fix, or for `MarkAnnotationsDecorative`
    /// ("the alt text of the screenshot describes the arrows and boxes").
    Confirm,
    /// The text the user typed, for `SetTitle` and `SetAltText`.
    Text(String),
    /// "Mark as decorative", for `SetAltText`.
    Decorative,
}

/// One fix that the user chose, for one finding.
#[derive(Debug, Clone)]
pub struct FixRequest {
    pub finding: Finding,
    pub input: FixInput,
    /// Which finding of its rule this is, counting from 0 in the order of the
    /// check. Two findings can be the same in every field (two pictures in one
    /// paragraph with no alt text), so the position tells them apart. When it
    /// is `None`, the first finding that matches and is not yet chosen is used.
    pub position: Option<usize>,
}

impl FixRequest {
    fn new(finding: &Finding, input: FixInput) -> FixRequest {
        FixRequest {
            finding: finding.clone(),
            input,
            position: None,
        }
    }

    pub fn confirm(finding: &Finding) -> FixRequest {
        FixRequest::new(finding, FixInput::Confirm)
    }

    pub fn text(finding: &Finding, text: &str) -> FixRequest {
        FixRequest::new(finding, FixInput::Text(text.to_string()))
    }

    pub fn decorative(finding: &Finding) -> FixRequest {
        FixRequest::new(finding, FixInput::Decorative)
    }

    /// The same request for the finding at this position among the findings of
    /// its rule.
    pub fn at(mut self, position: usize) -> FixRequest {
        self.position = Some(position);
        self
    }
}

/// Settings that change what a fix writes.
#[derive(Debug, Clone)]
pub struct FixOptions {
    /// The language `SetDefaultLanguage` writes (spec 03, DOC-002).
    pub default_language: String,
}

impl Default for FixOptions {
    fn default() -> FixOptions {
        FixOptions {
            default_language: "en-US".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppliedFix {
    pub finding: Finding,
    pub fix: FixId,
    /// What the tool did, in a sentence.
    pub summary: String,
}

/// A fix the tool did not apply, and why.
#[derive(Debug, Clone)]
pub struct Refusal {
    pub finding: Finding,
    /// The position of the request (see [`FixRequest::position`]).
    pub position: Option<usize>,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct Applied {
    /// The fixed .docx. `None` when no fix changed anything. No backup is
    /// needed, and nothing is written, in that case.
    pub bytes: Option<Vec<u8>>,
    pub applied: Vec<AppliedFix>,
    /// Findings whose fix is done when the PDF is exported (`UpdateFields`).
    /// Nothing is written for them.
    pub at_export: Vec<Finding>,
    pub refused: Vec<Refusal>,
    /// The splices that were applied, for each changed part, in text order.
    pub splices: BTreeMap<String, Vec<Splice>>,
}

/// The result of [`apply`] for a fix that the tool will not do.
type Reason = String;

enum Outcome {
    Done(String),
    AtExport,
}

/// What a fix does, in a phrase for a prompt.
pub fn describe(fix: FixId, options: &FixOptions) -> String {
    match fix {
        FixId::SetTitle => "Set the document title (File > Info > Title).".to_string(),
        FixId::SetDefaultLanguage => format!(
            "Set the document language to {}.",
            options.default_language
        ),
        FixId::ApplyHeadingStyle => "Apply the suggested heading style to this paragraph.".to_string(),
        FixId::DeleteEmptyHeading => "Delete this empty heading.".to_string(),
        FixId::ApplyListStyle => {
            "Remove the typed number or bullet and apply a list style.".to_string()
        }
        FixId::SetAltText => "Write alt text for this object, or mark it decorative.".to_string(),
        FixId::MakeImageInline => "Make this object in line with the text.".to_string(),
        FixId::MarkAnnotationsDecorative => {
            "Mark the arrows and boxes drawn on this screenshot as decorative.".to_string()
        }
        FixId::DeleteEmptyParagraphs => {
            "Delete the extra empty paragraphs (one stays).".to_string()
        }
        FixId::MarkHeaderRow => "Mark the first row as the header row.".to_string(),
        FixId::DeleteEmptyRow => "Delete the empty rows.".to_string(),
        FixId::DisallowRowBreak => "Do not allow rows to break across pages.".to_string(),
        FixId::AlignLeft => "Align this paragraph to the left.".to_string(),
        FixId::UpdateFields => {
            "Update the table of contents and other fields. Word does this when the PDF is exported.".to_string()
        }
    }
}

/// A part of the package that fixes read and splice.
#[derive(Clone, Copy)]
struct XmlPart<'a> {
    text: &'a str,
    xml: &'a roxmltree::Document<'a>,
}

/// Everything one run of fixes needs and collects.
struct Engine<'a> {
    doc: &'a Document,
    options: &'a FixOptions,
    document: XmlPart<'a>,
    styles: Option<XmlPart<'a>>,
    core: Option<XmlPart<'a>>,
    /// The `w:p` of every model paragraph, by the start of its byte range.
    paragraph_nodes: HashMap<usize, Node<'a, 'a>>,
    /// The `w:tbl` elements in the order of [`Document::tables`].
    table_nodes: Vec<Node<'a, 'a>>,
    splices: BTreeMap<&'static str, Splices>,
    paragraph_edits: BTreeMap<usize, ParagraphEdit>,
    row_edits: BTreeMap<(usize, usize), RowEdit>,
    /// Heading styles that the document has or that this run adds, by level.
    heading_styles: BTreeMap<u8, String>,
    /// The findings of each rule on the original document.
    fresh: HashMap<RuleId, Vec<Finding>>,
    claimed: HashSet<(RuleId, usize)>,
}

impl<'a> Engine<'a> {
    fn splices_for(&mut self, part: &'static str) -> &mut Splices {
        self.splices.entry(part).or_default()
    }

    /// Checks that the finding is still in the document and returns its
    /// position among the findings of its rule.
    fn position_of(&mut self, request: &FixRequest) -> Result<usize, Reason> {
        let finding = &request.finding;
        let rule = finding.rule_id;
        if !self.fresh.contains_key(&rule) {
            let found = rules::RULES
                .iter()
                .find(|(id, _)| *id == rule)
                .map(|(_, check)| check(self.doc))
                .unwrap_or_default();
            self.fresh.insert(rule, found);
        }
        let found = &self.fresh[&rule];
        let free = |i: usize| !self.claimed.contains(&(rule, i));
        let position = match request.position {
            Some(i) => (found.get(i) == Some(finding) && free(i)).then_some(i),
            None => found
                .iter()
                .enumerate()
                .position(|(i, f)| f == finding && free(i)),
        };
        match position {
            Some(i) => {
                self.claimed.insert((rule, i));
                Ok(i)
            }
            None => Err(
                "The document has changed since the check, or this fix was already chosen. Run the check again."
                    .to_string(),
            ),
        }
    }

    fn fix_one(&mut self, request: &FixRequest) -> Result<Outcome, Reason> {
        let finding = &request.finding;
        let id = match finding.fix {
            FixKind::Auto(id) | FixKind::Ask(id) => id,
            FixKind::Manual => {
                return Err(format!(
                    "The tool cannot fix this. Fix it in Word: {}",
                    finding.how_to_fix
                ));
            }
        };
        if id == FixId::UpdateFields {
            return Ok(Outcome::AtExport);
        }
        // The text of an ask fix is checked before anything else is looked at.
        let typed = match (id, &request.input) {
            (FixId::SetTitle | FixId::SetAltText, FixInput::Text(text)) => {
                if text.trim().is_empty() {
                    return Err("No text was typed, so nothing was written.".to_string());
                }
                Some(text.as_str())
            }
            (FixId::SetAltText, FixInput::Decorative) => None,
            (FixId::SetTitle, _) => return Err("This fix needs the text of the title.".to_string()),
            (FixId::SetAltText, _) => {
                return Err(
                    "This fix needs the alt text, or the choice to mark it decorative.".to_string(),
                );
            }
            (_, FixInput::Confirm) => None,
            (_, _) => return Err("This fix needs only the user's confirmation.".to_string()),
        };
        let position = self.position_of(request)?;
        let summary = match id {
            FixId::SetTitle => self.set_title(typed.unwrap_or_default())?,
            FixId::SetDefaultLanguage => self.set_default_language()?,
            FixId::ApplyHeadingStyle => self.apply_heading_style(finding)?,
            FixId::DeleteEmptyHeading => self.delete_empty_heading(finding)?,
            FixId::ApplyListStyle => self.apply_list_style(finding)?,
            FixId::SetAltText => self.set_alt_text(finding, position, typed)?,
            FixId::MakeImageInline => self.make_inline(position)?,
            FixId::MarkAnnotationsDecorative => self.mark_annotations_decorative(position)?,
            FixId::DeleteEmptyParagraphs => self.delete_empty_paragraphs(finding)?,
            FixId::MarkHeaderRow => self.mark_header_row(position)?,
            FixId::DeleteEmptyRow => self.delete_empty_rows(position)?,
            FixId::DisallowRowBreak => self.disallow_row_break(position)?,
            FixId::AlignLeft => self.align_left(finding)?,
            FixId::UpdateFields => return Ok(Outcome::AtExport),
        };
        Ok(Outcome::Done(summary))
    }

    /// The model paragraph and its element.
    fn paragraph(
        &self,
        index: usize,
    ) -> Result<(&'a crate::model::Paragraph, Node<'a, 'a>), Reason> {
        let paragraph = self
            .doc
            .paragraphs
            .get(index)
            .ok_or("The paragraph is not in the document.")?;
        let node = self
            .paragraph_nodes
            .get(&paragraph.range.start)
            .copied()
            .filter(|n| n.range() == paragraph.range)
            .ok_or("The paragraph could not be found in the file.")?;
        Ok((paragraph, node))
    }

    fn finding_paragraph(&self, finding: &Finding) -> Result<usize, Reason> {
        finding
            .location
            .paragraph_index
            .ok_or_else(|| "The finding has no paragraph.".to_string())
    }

    /// Turns the pending paragraph and row edits into splices.
    fn render_pending(&mut self) {
        let paragraph_edits = std::mem::take(&mut self.paragraph_edits);
        for (index, edit) in paragraph_edits {
            if let Ok((_, node)) = self.paragraph(index) {
                let text = self.document.text;
                paragraphs::render_paragraph_properties(
                    self.splices_for(DOCUMENT_PART),
                    text,
                    node,
                    &edit,
                );
            }
        }
        let row_edits = std::mem::take(&mut self.row_edits);
        for ((table, row), edit) in row_edits {
            if let Some(node) = self.row_node(table, row) {
                let text = self.document.text;
                tables::render_row(self.splices_for(DOCUMENT_PART), text, node, &edit);
            }
        }
    }

    fn part_text(&self, part: &str) -> Option<&'a str> {
        match part {
            DOCUMENT_PART => Some(self.document.text),
            STYLES_PART => self.styles.map(|p| p.text),
            CORE_PART => self.core.map(|p| p.text),
            _ => None,
        }
    }
}

/// Loads the document, runs the fixes and collects the results. `finish`
/// false stops before the new file is built: it is for [`offers`].
fn run(
    bytes: &[u8],
    requests: &[FixRequest],
    options: &FixOptions,
    finish: bool,
) -> Result<Applied, FixError> {
    let package = Package::from_bytes(bytes)?;
    let doc = Document::load(&package)?;
    let document_text = package
        .text(DOCUMENT_PART)?
        .ok_or_else(|| crate::error::Error::NotDocx(format!("it has no {DOCUMENT_PART}")))?;
    let document_xml = parse_xml(DOCUMENT_PART, document_text)?;
    let styles_xml = optional_xml(&package, STYLES_PART)?;
    let core_xml = optional_xml(&package, CORE_PART)?;
    let mut engine = Engine {
        doc: &doc,
        options,
        document: XmlPart {
            text: document_text,
            xml: &document_xml,
        },
        styles: styles_xml.as_ref().map(|(text, xml)| XmlPart { text, xml }),
        core: core_xml.as_ref().map(|(text, xml)| XmlPart { text, xml }),
        paragraph_nodes: document_xml
            .descendants()
            .filter(|n| n.is_w("p"))
            .map(|n| (n.range().start, n))
            .collect(),
        table_nodes: document_xml
            .root_element()
            .w_child("body")
            .map(|body| {
                body.descendants()
                    .filter(|n| n.is_w("tbl") && !xml::in_alternative_content(*n))
                    .collect()
            })
            .unwrap_or_default(),
        splices: BTreeMap::new(),
        paragraph_edits: BTreeMap::new(),
        row_edits: BTreeMap::new(),
        heading_styles: BTreeMap::new(),
        fresh: HashMap::new(),
        claimed: HashSet::new(),
    };

    let mut applied = Vec::new();
    let mut at_export = Vec::new();
    let mut refused = Vec::new();
    for request in requests {
        match engine.fix_one(request) {
            Ok(Outcome::Done(summary)) => applied.push(AppliedFix {
                finding: request.finding.clone(),
                fix: match request.finding.fix {
                    FixKind::Auto(id) | FixKind::Ask(id) => id,
                    FixKind::Manual => continue,
                },
                summary,
            }),
            Ok(Outcome::AtExport) => at_export.push(request.finding.clone()),
            Err(reason) => refused.push(Refusal {
                finding: request.finding.clone(),
                position: request.position,
                reason,
            }),
        }
    }
    let mut result = Applied {
        bytes: None,
        applied,
        at_export,
        refused,
        splices: BTreeMap::new(),
    };
    if !finish {
        return Ok(result);
    }

    engine.render_pending();
    let mut replacements = BTreeMap::new();
    for (name, splices) in &engine.splices {
        if splices.is_empty() {
            continue;
        }
        let Some(text) = engine.part_text(name) else {
            continue;
        };
        let resolved = splices.resolve().map_err(|_| FixError::Conflict)?;
        if resolved.is_empty() {
            continue;
        }
        let new_text = xml::apply_resolved(text, &resolved);
        parse_xml(name, &new_text).map_err(|e| FixError::Invalid(e.to_string()))?;
        let mut new_bytes = Vec::with_capacity(new_text.len() + 3);
        if package
            .part(name)
            .is_some_and(|b| b.starts_with(b"\xEF\xBB\xBF"))
        {
            new_bytes.extend_from_slice(b"\xEF\xBB\xBF");
        }
        new_bytes.extend_from_slice(new_text.as_bytes());
        replacements.insert((*name).to_string(), new_bytes);
        result.splices.insert((*name).to_string(), resolved);
    }
    if replacements.is_empty() {
        // Nothing changed, so every fix that "worked" did nothing.
        result.applied.clear();
        return Ok(result);
    }
    let new_bytes = package::rewrite(bytes, &replacements)?;
    Document::from_bytes(&new_bytes).map_err(|e| FixError::Invalid(e.to_string()))?;
    result.bytes = Some(new_bytes);
    Ok(result)
}

/// The text and the parsed XML of a part that may be missing.
fn optional_xml<'a>(
    package: &'a Package,
    name: &str,
) -> Result<Option<(&'a str, roxmltree::Document<'a>)>, FixError> {
    match package.text(name)? {
        Some(text) => Ok(Some((text, parse_xml(name, text)?))),
        None => Ok(None),
    }
}

/// Applies the chosen fixes to a .docx in memory. Nothing is written to disk.
///
/// All fixes are worked out against the original document and all splices are
/// applied to the original text, so the paragraph numbers in the findings stay
/// right when a fix deletes a paragraph. A fix that is refused is listed in
/// [`Applied::refused`] and changes nothing.
pub fn apply(
    bytes: &[u8],
    requests: &[FixRequest],
    options: &FixOptions,
) -> Result<Applied, FixError> {
    run(bytes, requests, options, true)
}

/// Whether a fix can be applied now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// The user can choose it.
    Ready,
    /// Nothing to do now: Word does it at export.
    AtExport,
    /// The tool will not do it. The reason says why, and what to do instead.
    Unavailable(String),
}

/// A finding that has a fix, and whether the fix can be applied.
#[derive(Debug, Clone)]
pub struct Offer {
    pub finding: Finding,
    /// The place of the finding among the findings of its rule.
    pub position: usize,
    pub fix: FixId,
    /// True when the user must type text or confirm something (an ask fix).
    pub ask: bool,
    pub availability: Availability,
}

impl Offer {
    /// The request that applies this fix with `input`.
    pub fn request(&self, input: FixInput) -> FixRequest {
        FixRequest {
            finding: self.finding.clone(),
            input,
            position: Some(self.position),
        }
    }
}

/// The findings that have an auto or ask fix, in the order of the report, with
/// whether the tool can apply each one.
pub fn offers(
    bytes: &[u8],
    options: &FixOptions,
    fonts: &dyn FontLookup,
) -> Result<Vec<Offer>, FixError> {
    let findings = crate::check_bytes(bytes, fonts)?;
    let mut seen: HashMap<RuleId, usize> = HashMap::new();
    let mut fixable: Vec<(Finding, usize, FixId, bool)> = Vec::new();
    for finding in findings {
        let position = {
            let count = seen.entry(finding.rule_id).or_insert(0);
            *count += 1;
            *count - 1
        };
        match finding.fix {
            FixKind::Auto(id) => fixable.push((finding, position, id, false)),
            FixKind::Ask(id) => fixable.push((finding, position, id, true)),
            FixKind::Manual => {}
        }
    }
    // The inputs are placeholders. Nothing is written: only the refusals count.
    let requests: Vec<FixRequest> = fixable
        .iter()
        .map(|(finding, position, id, asks)| FixRequest {
            finding: finding.clone(),
            input: match (asks, id) {
                (true, FixId::MarkAnnotationsDecorative) | (false, _) => FixInput::Confirm,
                (true, _) => FixInput::Text("placeholder".to_string()),
            },
            position: Some(*position),
        })
        .collect();
    let refused = run(bytes, &requests, options, false)?.refused;
    Ok(fixable
        .into_iter()
        .map(|(finding, position, fix, ask)| {
            let availability = if fix == FixId::UpdateFields {
                Availability::AtExport
            } else {
                match refused
                    .iter()
                    .find(|r| r.position == Some(position) && r.finding.rule_id == finding.rule_id)
                {
                    Some(refusal) => Availability::Unavailable(refusal.reason.clone()),
                    None => Availability::Ready,
                }
            };
            Offer {
                finding,
                position,
                fix,
                ask,
                availability,
            }
        })
        .collect())
}

/// What a run on a file needs from outside.
pub struct Environment<'a> {
    pub fs: &'a dyn FileSystem,
    /// The time in the name of the backup.
    pub now: Timestamp,
    /// For stage 1 after the fixes.
    pub fonts: &'a dyn FontLookup,
}

#[derive(Debug)]
pub struct FixOutcome {
    pub applied: Vec<AppliedFix>,
    pub at_export: Vec<Finding>,
    pub refused: Vec<Refusal>,
    /// The backup file. `None` when nothing was written.
    pub backup: Option<PathBuf>,
    /// Stage 1 on the file as it is now.
    pub findings: Vec<Finding>,
}

/// Checks that the file can be fixed: Word does not have it open and it can be
/// opened for writing. Nothing is written.
pub fn preflight(fs: &dyn FileSystem, path: &Path) -> Result<(), FixError> {
    open_for_fixing(fs, path).map(drop)
}

/// Fixes a .docx file on disk. The steps, in this order: look for the `~$` lock
/// file, open the file for exclusive writing, apply the fixes in memory, write
/// the backup, write the new file to a temporary file, move it over the
/// original. When no fix changes anything, nothing is written and there is no
/// backup. When a step fails, the original is as it was.
pub fn fix_file(
    path: &Path,
    requests: &[FixRequest],
    options: &FixOptions,
    env: &Environment,
) -> Result<FixOutcome, FixError> {
    let opened = open_for_fixing(env.fs, path)?;
    let applied = apply(&opened.bytes, requests, options)?;
    let (backup, findings) = match applied.bytes {
        Some(new) => {
            let findings = crate::check_bytes(&new, env.fonts)?;
            let backup = safety::write_with_backup(env.fs, path, opened, &new, env.now)?;
            (Some(backup), findings)
        }
        None => (None, crate::check_bytes(&opened.bytes, env.fonts)?),
    };
    Ok(FixOutcome {
        applied: applied.applied,
        at_export: applied.at_export,
        refused: applied.refused,
        backup,
        findings,
    })
}
