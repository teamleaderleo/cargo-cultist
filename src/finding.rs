use serde::{Deserialize, Serialize};

pub const REPORT_SCHEMA_VERSION: u32 = 1;

// Keep the full provenance vocabulary stable even while early analyzers only
// emit a subset of it. Future checks can add richer claims without changing
// the machine-readable taxonomy.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Proven,
    Derived,
    Observed,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

impl Location {
    pub fn new(path: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            path: path.into(),
            line,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl Evidence {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
        }
    }

    pub fn at(message: impl Into<String>, location: Location) -> Self {
        Self {
            message: message.into(),
            location: Some(location),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub kind: ClaimKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

impl Claim {
    pub fn new(kind: ClaimKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            evidence: Vec::new(),
        }
    }

    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub kind: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    pub claims: Vec<Claim>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

impl Finding {
    pub fn new(kind: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            title: title.into(),
            location: None,
            claims: Vec::new(),
            question: None,
        }
    }

    pub fn at(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_claim(mut self, claim: Claim) -> Self {
        self.claims.push(claim);
        self
    }

    pub fn with_question(mut self, question: impl Into<String>) -> Self {
        self.question = Some(question.into());
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisReport {
    pub schema_version: u32,
    pub analysis: String,
    pub repository: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
    pub findings: Vec<Finding>,
}

impl AnalysisReport {
    pub fn new(analysis: impl Into<String>, repository: impl Into<String>) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: analysis.into(),
            repository: repository.into(),
            claims: Vec::new(),
            findings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn serializes_claim_provenance() {
        let report = AnalysisReport {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: "example".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(ClaimKind::Observed, "example pattern")],
            findings: Vec::new(),
        };

        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["claims"][0]["kind"], "observed");
        assert_eq!(json["schema_version"], REPORT_SCHEMA_VERSION);
    }

    #[test]
    fn round_trips_machine_readable_report() {
        let report = AnalysisReport {
            schema_version: REPORT_SCHEMA_VERSION,
            analysis: "example".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(ClaimKind::Unknown, "missing evidence")],
            findings: vec![
                Finding::new("example", "Example")
                    .with_claim(Claim::new(ClaimKind::Proven, "exact fact")),
            ],
        };

        let json = serde_json::to_string(&report).unwrap();
        let decoded: AnalysisReport = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, report);
    }

    #[test]
    fn rejects_unknown_machine_report_fields_at_every_record_layer() {
        let base = sample_report_json();
        let mut cases = Vec::new();

        let mut report = base.clone();
        report
            .as_object_mut()
            .unwrap()
            .insert("future_report_field".to_string(), json!(true));
        cases.push(("report", report));

        let mut finding = base.clone();
        finding["findings"][0]
            .as_object_mut()
            .unwrap()
            .insert("future_finding_field".to_string(), json!(true));
        cases.push(("finding", finding));

        let mut claim = base.clone();
        claim["claims"][0]
            .as_object_mut()
            .unwrap()
            .insert("future_claim_field".to_string(), json!(true));
        cases.push(("claim", claim));

        let mut evidence = base.clone();
        evidence["claims"][0]["evidence"][0]
            .as_object_mut()
            .unwrap()
            .insert("future_evidence_field".to_string(), json!(true));
        cases.push(("evidence", evidence));

        let mut location = base;
        location["claims"][0]["evidence"][0]["location"]
            .as_object_mut()
            .unwrap()
            .insert("future_location_field".to_string(), json!(true));
        cases.push(("location", location));

        for (layer, value) in cases {
            let error = serde_json::from_value::<AnalysisReport>(value).unwrap_err();
            assert!(
                error.to_string().contains("unknown field"),
                "{layer} unexpectedly accepted unknown machine semantics: {error}"
            );
        }
    }

    fn sample_report_json() -> Value {
        json!({
            "schema_version": REPORT_SCHEMA_VERSION,
            "analysis": "example",
            "repository": "/repo",
            "claims": [
                {
                    "kind": "observed",
                    "message": "example observation",
                    "evidence": [
                        {
                            "message": "source evidence",
                            "location": {
                                "path": "src/lib.rs",
                                "line": 7
                            }
                        }
                    ]
                }
            ],
            "findings": [
                {
                    "kind": "example",
                    "title": "Example",
                    "location": {
                        "path": "src/main.rs"
                    },
                    "claims": [
                        {
                            "kind": "unknown",
                            "message": "missing discriminator"
                        }
                    ],
                    "question": "Investigate?"
                }
            ]
        })
    }
}
