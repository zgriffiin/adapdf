//! Adapdf checks Word training guides for accessibility problems.
//!
//! Stage 1 of the pipeline (spec 02): open a .docx, build a typed view of it
//! ([`model::Document`]), and run the pre-check rules ([`rules`]) to get
//! [`findings::Finding`] values.

use std::path::Path;

pub mod docx;
pub mod error;
pub mod findings;
pub mod fix;
pub mod fixtures;
pub mod fonts;
pub mod model;
pub mod render;
pub mod rules;

use error::Result;
use findings::Finding;
use fonts::FontLookup;

/// Checks a .docx file and returns every finding.
pub fn check_file(path: &Path, fonts: &dyn FontLookup) -> Result<Vec<Finding>> {
    let document = model::Document::open(path)?;
    Ok(rules::check(&document, fonts))
}

/// Checks a .docx that is already in memory.
pub fn check_bytes(bytes: &[u8], fonts: &dyn FontLookup) -> Result<Vec<Finding>> {
    let document = model::Document::from_bytes(bytes)?;
    Ok(rules::check(&document, fonts))
}
