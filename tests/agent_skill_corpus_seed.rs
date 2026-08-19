use std::collections::BTreeSet;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    description: String,
    episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Episode {
    id: String,
    ecosystem: Ecosystem,
    source_url: String,
    title: String,
    reported_boundary: ReportedBoundary,
    evidence_status: EvidenceStatus,
    #[serde(default)]
    source_evidence_url: Option<String>,
    #[serde(default)]
    source_evidence_blob_sha: Option<String>,
    #[serde(default)]
    repair_url: Option<String>,
    #[serde(default)]
    repair_status: Option<RepairStatus>,
    candidate_discriminator: String,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Ecosystem {
    Anthropic,
    Openai,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReportedBoundary {
    MeasurementMisrepresented,
    SelectionBudgetOverreach,
    ContextBindingMiss,
    AffordanceMiss,
    ProvenanceIdentityGap,
    SchemaToolingDrift,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceStatus {
    ReportedOnly,
    SourceMechanismConfirmed,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RepairStatus {
    ProposedOpen,
}

fn is_lower_hex_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[test]
fn agent_skill_seed_is_bounded_cross_ecosystem_and_keeps_evidence_state_explicit() {
    let corpus: Corpus =
        serde_json::from_slice(include_bytes!("../research/agent-skill-corpus-seed.json"))
            .expect("valid corpus JSON");

    assert_eq!(corpus.schema_version, 1);
    assert!(corpus.description.contains("Evidence status"));
    assert!(corpus.episodes.len() >= 12);
    assert!(corpus.episodes.len() <= 64);

    let mut ids = BTreeSet::new();
    let mut ecosystems = BTreeSet::new();
    let mut boundaries = BTreeSet::new();
    let mut source_confirmed = 0usize;

    for episode in &corpus.episodes {
        assert!(
            ids.insert(episode.id.as_str()),
            "duplicate id {}",
            episode.id
        );
        ecosystems.insert(episode.ecosystem);
        boundaries.insert(episode.reported_boundary);

        assert!(!episode.title.trim().is_empty());
        assert!(!episode.candidate_discriminator.trim().is_empty());

        let (issue_prefix, blob_prefix, pull_prefix) = match episode.ecosystem {
            Ecosystem::Anthropic => (
                "https://github.com/anthropics/skills/issues/",
                "https://github.com/anthropics/skills/blob/",
                "https://github.com/anthropics/skills/pull/",
            ),
            Ecosystem::Openai => (
                "https://github.com/openai/skills/issues/",
                "https://github.com/openai/skills/blob/",
                "https://github.com/openai/skills/pull/",
            ),
        };
        assert!(
            episode.source_url.starts_with(issue_prefix),
            "unexpected source URL {}",
            episode.source_url
        );

        let issue_number = episode
            .id
            .rsplit_once('#')
            .map(|(_, number)| number)
            .expect("id contains issue number");
        assert!(
            episode.source_url.ends_with(issue_number),
            "source URL does not match id {}",
            episode.id
        );

        match episode.evidence_status {
            EvidenceStatus::ReportedOnly => {
                assert!(episode.source_evidence_url.is_none());
                assert!(episode.source_evidence_blob_sha.is_none());
                assert!(episode.repair_url.is_none());
                assert!(episode.repair_status.is_none());
            }
            EvidenceStatus::SourceMechanismConfirmed => {
                source_confirmed += 1;
                let source_evidence_url = episode
                    .source_evidence_url
                    .as_deref()
                    .expect("confirmed mechanism has exact source URL");
                let source_evidence_blob_sha = episode
                    .source_evidence_blob_sha
                    .as_deref()
                    .expect("confirmed mechanism has source blob SHA");
                assert!(source_evidence_url.starts_with(blob_prefix));
                assert!(is_lower_hex_sha(source_evidence_blob_sha));

                let repair_url = episode
                    .repair_url
                    .as_deref()
                    .expect("confirmed mechanism records known repair proposal");
                assert!(repair_url.starts_with(pull_prefix));
                assert_eq!(episode.repair_status, Some(RepairStatus::ProposedOpen));
            }
        }
    }

    assert_eq!(
        ecosystems.len(),
        2,
        "seed must span both sampled ecosystems"
    );
    assert!(
        boundaries.len() >= 5,
        "seed should test multiple candidate failure species"
    );
    assert!(
        source_confirmed >= 1,
        "at least one reported diagnosis should graduate through source evidence"
    );
}
