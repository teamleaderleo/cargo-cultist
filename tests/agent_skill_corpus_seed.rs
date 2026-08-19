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
}

#[test]
fn reported_agent_skill_seed_is_bounded_cross_ecosystem_and_explicitly_unverified() {
    let corpus: Corpus = serde_json::from_slice(include_bytes!(
        "../research/agent-skill-corpus-seed.json"
    ))
    .expect("valid corpus JSON");

    assert_eq!(corpus.schema_version, 1);
    assert!(corpus.description.contains("claims"));
    assert!(corpus.episodes.len() >= 12);
    assert!(corpus.episodes.len() <= 64);

    let mut ids = BTreeSet::new();
    let mut ecosystems = BTreeSet::new();
    let mut boundaries = BTreeSet::new();

    for episode in &corpus.episodes {
        assert!(ids.insert(episode.id.as_str()), "duplicate id {}", episode.id);
        ecosystems.insert(episode.ecosystem);
        boundaries.insert(episode.reported_boundary);

        assert_eq!(episode.evidence_status, EvidenceStatus::ReportedOnly);
        assert!(!episode.title.trim().is_empty());
        assert!(!episode.candidate_discriminator.trim().is_empty());

        let expected_prefix = match episode.ecosystem {
            Ecosystem::Anthropic => "https://github.com/anthropics/skills/issues/",
            Ecosystem::Openai => "https://github.com/openai/skills/issues/",
        };
        assert!(
            episode.source_url.starts_with(expected_prefix),
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
    }

    assert_eq!(ecosystems.len(), 2, "seed must span both sampled ecosystems");
    assert!(
        boundaries.len() >= 5,
        "seed should test multiple candidate failure species"
    );
}
