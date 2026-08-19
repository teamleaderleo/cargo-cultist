use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::behavioral_episode::{BehavioralEpisodeBatch, validate_behavioral_episode_batch};
use crate::behavioral_receipt::{BehavioralOutcome, Delivery};

pub const BEHAVIORAL_SUMMARY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralCount {
    pub key: String,
    pub count: usize,
    pub episode_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralSummary {
    pub schema_version: u32,
    pub total_episodes: usize,
    pub surfaced: usize,
    pub quiet: usize,
    pub consulted: usize,
    pub by_outcome: Vec<BehavioralCount>,
    pub by_evidence_kind: Vec<BehavioralCount>,
}

pub fn summarize_behavioral_episodes(
    batch: &BehavioralEpisodeBatch,
) -> Result<BehavioralSummary, String> {
    validate_behavioral_episode_batch(batch).map_err(|error| error.to_string())?;

    let mut surfaced = 0usize;
    let mut quiet = 0usize;
    let mut consulted = 0usize;
    let mut by_outcome = BTreeMap::<String, Vec<String>>::new();
    let mut by_evidence_kind = BTreeMap::<String, Vec<String>>::new();

    for episode in &batch.episodes {
        match episode.receipt.delivery {
            Delivery::Surfaced => surfaced += 1,
            Delivery::Quiet => quiet += 1,
        }
        if episode.receipt.consulted {
            consulted += 1;
        }

        by_outcome
            .entry(outcome_name(episode.receipt.outcome).to_string())
            .or_default()
            .push(episode.episode_id.clone());
        by_evidence_kind
            .entry(episode.receipt.evidence_kind.clone())
            .or_default()
            .push(episode.episode_id.clone());
    }

    Ok(BehavioralSummary {
        schema_version: BEHAVIORAL_SUMMARY_SCHEMA_VERSION,
        total_episodes: batch.episodes.len(),
        surfaced,
        quiet,
        consulted,
        by_outcome: counts(by_outcome),
        by_evidence_kind: counts(by_evidence_kind),
    })
}

fn counts(values: BTreeMap<String, Vec<String>>) -> Vec<BehavioralCount> {
    values
        .into_iter()
        .map(|(key, mut episode_ids)| {
            episode_ids.sort();
            BehavioralCount {
                key,
                count: episode_ids.len(),
                episode_ids,
            }
        })
        .collect()
}

fn outcome_name(outcome: BehavioralOutcome) -> &'static str {
    match outcome {
        BehavioralOutcome::ChangedNextAction => "changed_next_action",
        BehavioralOutcome::PreventedOrReversedWrongTurn => "prevented_or_reversed_wrong_turn",
        BehavioralOutcome::UsefulSameAction => "useful_same_action",
        BehavioralOutcome::Ignored => "ignored",
        BehavioralOutcome::Irrelevant => "irrelevant",
        BehavioralOutcome::StaleOrWrongCoordinate => "stale_or_wrong_coordinate",
        BehavioralOutcome::NeededStrongerEvidence => "needed_stronger_evidence",
        BehavioralOutcome::CorrectQuietNegative => "correct_quiet_negative",
    }
}
