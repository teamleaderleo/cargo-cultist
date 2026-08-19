use std::error::Error;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::compact_ir::{CompactError, encode_report};
use crate::finding::AnalysisReport;

pub const REPORT_FINGERPRINT_SCHEME: &str = "cultist-report-c1-sha256-v1";
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ReportFingerprint(String);

impl ReportFingerprint {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ReportFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for ReportFingerprint {
    type Err = ReportFingerprintError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let expected_prefix = format!("{REPORT_FINGERPRINT_SCHEME}:");
        let digest =
            value
                .strip_prefix(&expected_prefix)
                .ok_or(ReportFingerprintError::InvalidFormat(
                    "unsupported report fingerprint scheme",
                ))?;

        if digest.len() != SHA256_HEX_BYTES {
            return Err(ReportFingerprintError::InvalidFormat(
                "SHA-256 report fingerprint must contain exactly 64 hex characters",
            ));
        }
        if !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ReportFingerprintError::InvalidFormat(
                "report fingerprint digest must use lowercase hexadecimal",
            ));
        }

        Ok(Self(value.to_string()))
    }
}

impl Serialize for ReportFingerprint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ReportFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum ReportFingerprintError {
    Compact(CompactError),
    InvalidFormat(&'static str),
}

impl fmt::Display for ReportFingerprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compact(error) => write!(formatter, "cannot fingerprint report: {error}"),
            Self::InvalidFormat(message) => formatter.write_str(message),
        }
    }
}

impl Error for ReportFingerprintError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Compact(error) => Some(error),
            Self::InvalidFormat(_) => None,
        }
    }
}

impl From<CompactError> for ReportFingerprintError {
    fn from(error: CompactError) -> Self {
        Self::Compact(error)
    }
}

pub fn fingerprint_report(
    report: &AnalysisReport,
) -> Result<ReportFingerprint, ReportFingerprintError> {
    let canonical_c1 = encode_report(report)?;
    let digest = Sha256::digest(canonical_c1.as_bytes());
    let mut hex = String::with_capacity(SHA256_HEX_BYTES);
    for byte in digest {
        hex.push(hex_digit(byte >> 4));
        hex.push(hex_digit(byte & 0x0f));
    }

    Ok(ReportFingerprint(format!(
        "{REPORT_FINGERPRINT_SCHEME}:{hex}"
    )))
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => unreachable!("nibble is masked to four bits"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compact_ir::decode_report;
    use crate::finding::{Claim, ClaimKind, Evidence, Finding, Location, REPORT_SCHEMA_VERSION};

    fn small_report() -> AnalysisReport {
        AnalysisReport {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: "a".to_string(),
            repository: "r".to_string(),
            claims: vec![Claim::new(ClaimKind::Observed, "m")],
            findings: Vec::new(),
        }
    }

    fn representative_report() -> AnalysisReport {
        AnalysisReport {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: "preflight-inventory".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(
                ClaimKind::Unknown,
                "semantic independence is unresolved",
            )],
            findings: vec![
                Finding::new("direct-overlap", "Direct path overlap")
                    .at(Location::new("src/auth.rs", Some(12)))
                    .with_claim(
                        Claim::new(ClaimKind::Proven, "both work items modify src/auth.rs")
                            .with_evidence(Evidence::new("github:pull/10")),
                    )
                    .with_question("Coordinate ownership?"),
                Finding::new("explicit-coordination", "Explicit coordination").with_claim(
                    Claim::new(ClaimKind::Observed, "hold_merge_while #10 > #11"),
                ),
            ],
        }
    }

    #[test]
    fn exact_small_report_has_stable_known_fingerprint() {
        let fingerprint = fingerprint_report(&small_report()).unwrap();
        assert_eq!(
            fingerprint.as_str(),
            "cultist-report-c1-sha256-v1:6f26b3b5673e76878bc3f0d575d8a8347543920cd50c3a92b5eb18d23c0dbfa6"
        );
    }

    #[test]
    fn repeated_fingerprints_are_identical() {
        let report = representative_report();
        assert_eq!(
            fingerprint_report(&report).unwrap(),
            fingerprint_report(&report).unwrap()
        );
    }

    #[test]
    fn json_formatting_does_not_change_typed_report_fingerprint() {
        let report = representative_report();
        let minified = serde_json::to_string(&report).unwrap();
        let pretty = serde_json::to_string_pretty(&report).unwrap();
        assert_ne!(minified, pretty);

        let from_minified: AnalysisReport = serde_json::from_str(&minified).unwrap();
        let from_pretty: AnalysisReport = serde_json::from_str(&pretty).unwrap();
        assert_eq!(
            fingerprint_report(&from_minified).unwrap(),
            fingerprint_report(&from_pretty).unwrap()
        );
    }

    #[test]
    fn c1_round_trip_preserves_fingerprint() {
        let report = representative_report();
        let c1 = encode_report(&report).unwrap();
        let decoded = decode_report(&c1).unwrap();
        assert_eq!(
            fingerprint_report(&report).unwrap(),
            fingerprint_report(&decoded).unwrap()
        );
    }

    #[test]
    fn finding_reorder_changes_exact_snapshot_fingerprint() {
        let report = representative_report();
        let mut reordered = report.clone();
        reordered.findings.reverse();
        assert_ne!(
            fingerprint_report(&report).unwrap(),
            fingerprint_report(&reordered).unwrap()
        );
    }

    #[test]
    fn semantic_snapshot_mutations_change_fingerprint() {
        let report = representative_report();

        let mut claim = report.clone();
        claim.findings[0].claims[0].message.push('!');

        let mut evidence = report.clone();
        evidence.findings[0].claims[0].evidence[0].message.push('!');

        let mut question = report.clone();
        question.findings[0].question = Some("Different question?".to_string());

        let mut location = report.clone();
        location.findings[0].location.as_mut().unwrap().line = Some(13);

        let baseline = fingerprint_report(&report).unwrap();
        for changed in [claim, evidence, question, location] {
            assert_ne!(baseline, fingerprint_report(&changed).unwrap());
        }
    }

    #[test]
    fn unsupported_report_schema_rejects_before_hashing() {
        let mut report = small_report();
        report.schema_version += 1;
        assert!(matches!(
            fingerprint_report(&report),
            Err(ReportFingerprintError::Compact(_))
        ));
    }

    #[test]
    fn fingerprint_text_parser_fails_closed() {
        let valid = fingerprint_report(&small_report()).unwrap();
        assert_eq!(valid.as_str().parse::<ReportFingerprint>().unwrap(), valid);

        let wrong_scheme = valid.as_str().replacen(
            REPORT_FINGERPRINT_SCHEME,
            "cultist-report-json-sha256-v1",
            1,
        );
        assert!(wrong_scheme.parse::<ReportFingerprint>().is_err());

        let short = format!("{REPORT_FINGERPRINT_SCHEME}:abcd");
        assert!(short.parse::<ReportFingerprint>().is_err());

        let uppercase = valid.as_str().to_ascii_uppercase();
        assert!(uppercase.parse::<ReportFingerprint>().is_err());

        let non_hex = format!("{REPORT_FINGERPRINT_SCHEME}:{}g", "0".repeat(63));
        assert!(non_hex.parse::<ReportFingerprint>().is_err());
    }

    #[test]
    fn fingerprint_serde_revalidates_wire_value() {
        let fingerprint = fingerprint_report(&small_report()).unwrap();
        let json = serde_json::to_string(&fingerprint).unwrap();
        let decoded: ReportFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, fingerprint);

        let invalid = format!("\"{REPORT_FINGERPRINT_SCHEME}:abcd\"");
        assert!(serde_json::from_str::<ReportFingerprint>(&invalid).is_err());
    }
}
