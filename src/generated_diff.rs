use std::error::Error;
use std::path::Path;

use crate::finding::AnalysisReport;

/// Generated-companion findings are temporarily fail-closed while #80 repairs
/// repository-root provenance in the product ownership extractor.
///
/// The held-out Oxc evidence remains preserved in research receipts. Product
/// findings resume only after the canonical ownership path rejects arbitrary
/// `receiver.join("literal")` expressions and re-passes the positive/negative
/// product replay.
pub fn add_generated_companion_findings(
    _root: &Path,
    _base: Option<&str>,
    _analysis: &mut AnalysisReport,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
