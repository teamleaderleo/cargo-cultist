use std::error::Error;
use std::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::finding::{
    AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location, REPORT_SCHEMA_VERSION,
};

const GRAMMAR_HEADER: &str = "C1";
pub const MAX_C1_BYTES: usize = 1024 * 1024;
pub const MAX_C1_RECORDS: usize = 4096;
pub const MAX_C1_RECORD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompactError {
    line: Option<usize>,
    message: String,
}

impl CompactError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
        }
    }

    fn at(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }
}

impl fmt::Display for CompactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "C1 line {line}: {}", self.message),
            None => write!(formatter, "C1: {}", self.message),
        }
    }
}

impl Error for CompactError {}

#[derive(Debug, Clone, Copy)]
enum ClaimTarget {
    Top(usize),
    Finding(usize, usize),
}

type WireLocation = Option<(String, Option<usize>)>;

pub fn encode_report(report: &AnalysisReport) -> Result<String, CompactError> {
    validate_report_schema(report.schema_version, None)?;

    let mut output = String::new();
    output.push_str(GRAMMAR_HEADER);
    output.push('\n');

    push_record(
        &mut output,
        'R',
        &(
            report.schema_version,
            report.analysis.as_str(),
            report.repository.as_str(),
        ),
    )?;

    for claim in &report.claims {
        encode_claim(&mut output, 'C', claim)?;
    }

    for finding in &report.findings {
        push_record(
            &mut output,
            'F',
            &(
                finding.kind.as_str(),
                finding.title.as_str(),
                location_ref(finding.location.as_ref()),
            ),
        )?;

        for claim in &finding.claims {
            encode_claim(&mut output, 'c', claim)?;
        }

        if let Some(question) = &finding.question {
            push_record(&mut output, 'q', &(question.as_str(),))?;
        }
    }

    validate_transport_bounds(&output)?;
    Ok(output)
}

pub fn decode_report(input: &str) -> Result<AnalysisReport, CompactError> {
    validate_transport_bounds(input)?;

    let mut lines = input.lines().enumerate();
    let Some((_, header)) = lines.next() else {
        return Err(CompactError::new("missing grammar header"));
    };
    if header != GRAMMAR_HEADER {
        return Err(CompactError::at(
            1,
            format!("unsupported grammar header `{header}`; expected `{GRAMMAR_HEADER}`"),
        ));
    }

    let mut report: Option<AnalysisReport> = None;
    let mut current_finding: Option<usize> = None;
    let mut last_claim: Option<ClaimTarget> = None;
    let mut question_seen = false;

    for (zero_based, line) in lines {
        let line_number = zero_based + 1;
        if line.is_empty() {
            return Err(CompactError::at(
                line_number,
                "empty records are not allowed",
            ));
        }

        let mut chars = line.chars();
        let tag = chars.next().expect("non-empty line");
        let payload = chars.as_str();

        match tag {
            'R' => {
                if line_number != 2 || report.is_some() {
                    return Err(CompactError::at(
                        line_number,
                        "report identity must be the first and only R record",
                    ));
                }
                let (schema_version, analysis, repository): (u32, String, String) =
                    parse_payload(payload, line_number)?;
                validate_report_schema(schema_version, Some(line_number))?;
                report = Some(AnalysisReport {
                    schema_version,
                    analysis,
                    repository,
                    claims: Vec::new(),
                    findings: Vec::new(),
                });
            }
            'C' => {
                let report = report
                    .as_mut()
                    .ok_or_else(|| CompactError::at(line_number, "C record before R record"))?;
                if current_finding.is_some() {
                    return Err(CompactError::at(
                        line_number,
                        "top-level C records must precede all F records",
                    ));
                }
                let claim = parse_claim(payload, line_number)?;
                report.claims.push(claim);
                last_claim = Some(ClaimTarget::Top(report.claims.len() - 1));
            }
            'F' => {
                let report = report
                    .as_mut()
                    .ok_or_else(|| CompactError::at(line_number, "F record before R record"))?;
                let (kind, title, location): (String, String, WireLocation) =
                    parse_payload(payload, line_number)?;
                report.findings.push(Finding {
                    kind,
                    title,
                    location: location_from_wire(location),
                    claims: Vec::new(),
                    question: None,
                });
                current_finding = Some(report.findings.len() - 1);
                last_claim = None;
                question_seen = false;
            }
            'c' => {
                let report = report
                    .as_mut()
                    .ok_or_else(|| CompactError::at(line_number, "c record before R record"))?;
                let finding_index = current_finding.ok_or_else(|| {
                    CompactError::at(line_number, "finding claim c record outside an F record")
                })?;
                if question_seen {
                    return Err(CompactError::at(
                        line_number,
                        "finding claims may not follow a q record",
                    ));
                }
                let claim = parse_claim(payload, line_number)?;
                report.findings[finding_index].claims.push(claim);
                let claim_index = report.findings[finding_index].claims.len() - 1;
                last_claim = Some(ClaimTarget::Finding(finding_index, claim_index));
            }
            'e' => {
                let report = report
                    .as_mut()
                    .ok_or_else(|| CompactError::at(line_number, "e record before R record"))?;
                let target = last_claim.ok_or_else(|| {
                    CompactError::at(line_number, "e record must immediately belong to a claim")
                })?;
                let (message, location): (String, WireLocation) =
                    parse_payload(payload, line_number)?;
                let evidence = Evidence {
                    message,
                    location: location_from_wire(location),
                };
                match target {
                    ClaimTarget::Top(claim_index) => {
                        report.claims[claim_index].evidence.push(evidence);
                    }
                    ClaimTarget::Finding(finding_index, claim_index) => {
                        report.findings[finding_index].claims[claim_index]
                            .evidence
                            .push(evidence);
                    }
                }
            }
            'q' => {
                let report = report
                    .as_mut()
                    .ok_or_else(|| CompactError::at(line_number, "q record before R record"))?;
                let finding_index = current_finding
                    .ok_or_else(|| CompactError::at(line_number, "q record outside an F record"))?;
                if question_seen {
                    return Err(CompactError::at(
                        line_number,
                        "finding may contain at most one q record",
                    ));
                }
                let (question,): (String,) = parse_payload(payload, line_number)?;
                report.findings[finding_index].question = Some(question);
                question_seen = true;
                last_claim = None;
            }
            other => {
                return Err(CompactError::at(
                    line_number,
                    format!("unknown record tag `{other}`"),
                ));
            }
        }
    }

    report.ok_or_else(|| CompactError::new("missing R record"))
}

fn encode_claim(output: &mut String, tag: char, claim: &Claim) -> Result<(), CompactError> {
    push_record(
        output,
        tag,
        &(claim_kind_code(claim.kind), claim.message.as_str()),
    )?;
    for evidence in &claim.evidence {
        push_record(
            output,
            'e',
            &(
                evidence.message.as_str(),
                location_ref(evidence.location.as_ref()),
            ),
        )?;
    }
    Ok(())
}

fn push_record<T: Serialize + ?Sized>(
    output: &mut String,
    tag: char,
    payload: &T,
) -> Result<(), CompactError> {
    let payload = serde_json::to_string(payload)
        .map_err(|error| CompactError::new(format!("could not serialize record: {error}")))?;
    output.push(tag);
    output.push_str(&payload);
    output.push('\n');
    Ok(())
}

fn validate_report_schema(
    schema_version: u32,
    line: Option<usize>,
) -> Result<(), CompactError> {
    if schema_version == REPORT_SCHEMA_VERSION {
        return Ok(());
    }

    let message = format!(
        "unsupported AnalysisReport schema {schema_version}; expected {REPORT_SCHEMA_VERSION}"
    );
    Err(match line {
        Some(line) => CompactError::at(line, message),
        None => CompactError::new(message),
    })
}

fn validate_transport_bounds(input: &str) -> Result<(), CompactError> {
    if input.len() > MAX_C1_BYTES {
        return Err(CompactError::new(format!(
            "input is {} bytes; maximum C1 size is {MAX_C1_BYTES} bytes",
            input.len()
        )));
    }

    let mut records = 0usize;
    for (zero_based, line) in input.lines().enumerate() {
        let line_number = zero_based + 1;
        if line.len() > MAX_C1_RECORD_BYTES {
            return Err(CompactError::at(
                line_number,
                format!(
                    "record is {} bytes; maximum record size is {MAX_C1_RECORD_BYTES} bytes",
                    line.len()
                ),
            ));
        }
        if line_number > 1 {
            records += 1;
            if records > MAX_C1_RECORDS {
                return Err(CompactError::at(
                    line_number,
                    format!("record count exceeds maximum {MAX_C1_RECORDS}"),
                ));
            }
        }
    }

    Ok(())
}

fn parse_claim(payload: &str, line: usize) -> Result<Claim, CompactError> {
    let (code, message): (String, String) = parse_payload(payload, line)?;
    let kind = parse_claim_kind(&code)
        .ok_or_else(|| CompactError::at(line, format!("unknown claim code `{code}`")))?;
    Ok(Claim {
        kind,
        message,
        evidence: Vec::new(),
    })
}

fn parse_payload<T: DeserializeOwned>(payload: &str, line: usize) -> Result<T, CompactError> {
    serde_json::from_str(payload)
        .map_err(|error| CompactError::at(line, format!("invalid record payload: {error}")))
}

fn claim_kind_code(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Proven => "P",
        ClaimKind::Derived => "D",
        ClaimKind::Observed => "O",
        ClaimKind::Inferred => "I",
        ClaimKind::Unknown => "U",
    }
}

fn parse_claim_kind(code: &str) -> Option<ClaimKind> {
    match code {
        "P" => Some(ClaimKind::Proven),
        "D" => Some(ClaimKind::Derived),
        "O" => Some(ClaimKind::Observed),
        "I" => Some(ClaimKind::Inferred),
        "U" => Some(ClaimKind::Unknown),
        _ => None,
    }
}

fn location_ref(location: Option<&Location>) -> Option<(&str, Option<usize>)> {
    location.map(|location| (location.path.as_str(), location.line))
}

fn location_from_wire(location: WireLocation) -> Option<Location> {
    location.map(|(path, line)| Location { path, line })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_report() -> AnalysisReport {
        AnalysisReport {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: "preflight-inventory".to_string(),
            repository: "/tmp/repo with spaces".to_string(),
            claims: vec![
                Claim::new(ClaimKind::Proven, "exact fact"),
                Claim::new(ClaimKind::Derived, "derived relationship"),
                Claim::new(ClaimKind::Observed, "observed pattern").with_evidence(Evidence::at(
                    "source says \"hello\"\nwith a newline",
                    Location::new("src/lib.rs", Some(42)),
                )),
                Claim::new(ClaimKind::Inferred, "plausible interpretation"),
                Claim::new(ClaimKind::Unknown, "missing discriminator"),
            ],
            findings: vec![
                Finding::new("explicit-coordination", "Explicit coordination")
                    .at(Location::new("src/main.rs", None))
                    .with_claim(
                        Claim::new(ClaimKind::Observed, "hold_merge_while #748 > #703")
                            .with_evidence(Evidence::new("github:pull/748")),
                    )
                    .with_claim(Claim::new(
                        ClaimKind::Unknown,
                        "operational consequence beyond declared relation",
                    ))
                    .with_question("Coordinate merge order?"),
            ],
        }
    }

    #[test]
    fn exact_small_encoding_is_stable() {
        let report = AnalysisReport {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: "a".to_string(),
            repository: "r".to_string(),
            claims: vec![Claim::new(ClaimKind::Observed, "m")],
            findings: Vec::new(),
        };

        assert_eq!(
            encode_report(&report).unwrap(),
            "C1\nR[1,\"a\",\"r\"]\nC[\"O\",\"m\"]\n"
        );
    }

    #[test]
    fn round_trips_every_claim_kind_and_nested_evidence() {
        let report = full_report();
        let encoded = encode_report(&report).unwrap();
        let decoded = decode_report(&encoded).unwrap();
        assert_eq!(decoded, report);
    }

    #[test]
    fn encoding_is_deterministic() {
        let report = full_report();
        assert_eq!(
            encode_report(&report).unwrap(),
            encode_report(&report).unwrap()
        );
    }

    #[test]
    fn c1_is_smaller_than_minified_json_for_representative_report() {
        let report = full_report();
        let json = serde_json::to_string(&report).unwrap();
        let c1 = encode_report(&report).unwrap();
        assert!(
            c1.len() < json.len(),
            "expected C1 ({} bytes) < JSON ({} bytes)",
            c1.len(),
            json.len()
        );
    }

    #[test]
    fn rejects_unknown_grammar_header() {
        assert!(decode_report("C2\nR[1,\"a\",\"r\"]\n").is_err());
    }

    #[test]
    fn rejects_unknown_report_schema_on_encode_and_decode() {
        let mut report = full_report();
        report.schema_version = REPORT_SCHEMA_VERSION + 1;
        let encode_error = encode_report(&report).unwrap_err();
        assert!(encode_error.to_string().contains("unsupported AnalysisReport schema"));

        let input = format!(
            "C1\nR[{},\"a\",\"r\"]\n",
            REPORT_SCHEMA_VERSION + 1
        );
        let decode_error = decode_report(&input).unwrap_err();
        assert!(decode_error.to_string().contains("unsupported AnalysisReport schema"));
    }

    #[test]
    fn rejects_input_over_total_byte_budget() {
        let input = "x".repeat(MAX_C1_BYTES + 1);
        let error = decode_report(&input).unwrap_err();
        assert!(error.to_string().contains("maximum C1 size"));
    }

    #[test]
    fn rejects_record_over_byte_budget() {
        let message = "x".repeat(MAX_C1_RECORD_BYTES);
        let input = format!("C1\nR[1,\"a\",\"r\"]\nC[\"O\",\"{message}\"]\n");
        let error = decode_report(&input).unwrap_err();
        assert!(error.to_string().contains("maximum record size"));
    }

    #[test]
    fn rejects_too_many_records() {
        let mut input = "C1\nR[1,\"a\",\"r\"]\n".to_string();
        for _ in 0..MAX_C1_RECORDS {
            input.push_str("C[\"O\",\"m\"]\n");
        }
        let error = decode_report(&input).unwrap_err();
        assert!(error.to_string().contains("record count exceeds maximum"));
    }

    #[test]
    fn encoder_does_not_emit_packet_outside_decoder_bounds() {
        let mut report = full_report();
        report.claims = vec![Claim::new(
            ClaimKind::Observed,
            "x".repeat(MAX_C1_RECORD_BYTES),
        )];
        assert!(encode_report(&report).is_err());
    }

    #[test]
    fn rejects_unknown_record_tag() {
        assert!(decode_report("C1\nR[1,\"a\",\"r\"]\nX[]\n").is_err());
    }

    #[test]
    fn rejects_unknown_claim_code() {
        assert!(decode_report("C1\nR[1,\"a\",\"r\"]\nC[\"Z\",\"m\"]\n").is_err());
    }

    #[test]
    fn rejects_evidence_without_claim() {
        assert!(decode_report("C1\nR[1,\"a\",\"r\"]\ne[\"m\",null]\n").is_err());
    }

    #[test]
    fn rejects_question_without_finding() {
        assert!(decode_report("C1\nR[1,\"a\",\"r\"]\nq[\"why?\"]\n").is_err());
    }

    #[test]
    fn rejects_top_level_claim_after_finding() {
        assert!(
            decode_report("C1\nR[1,\"a\",\"r\"]\nF[\"k\",\"t\",null]\nC[\"O\",\"m\"]\n").is_err()
        );
    }

    #[test]
    fn rejects_claim_after_question() {
        assert!(
            decode_report(
                "C1\nR[1,\"a\",\"r\"]\nF[\"k\",\"t\",null]\nq[\"why?\"]\nc[\"O\",\"m\"]\n"
            )
            .is_err()
        );
    }
}
