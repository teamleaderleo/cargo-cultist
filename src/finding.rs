use serde::Serialize;

pub const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Proven,
    Derived,
    Observed,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
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
}
