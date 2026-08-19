use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const BEHAVIORAL_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const MAX_BEHAVIORAL_RECEIPT_BYTES: usize = 64 * 1024;
const MAX_COORDINATE_BYTES: usize = 1024;
const MAX_ACTION_BYTES: usize = 2048;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralReceipt {
    pub schema_version: u32,
    pub repository: String,
    pub revision: String,
    pub task: String,
    pub evidence_kind: String,
    pub evidence_ref: String,
    pub delivery: Delivery,
    pub consulted: bool,
    pub outcome: BehavioralOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    Surfaced,
    Quiet,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralOutcome {
    ChangedNextAction,
    PreventedOrReversedWrongTurn,
    UsefulSameAction,
    Ignored,
    Irrelevant,
    StaleOrWrongCoordinate,
    NeededStrongerEvidence,
    CorrectQuietNegative,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BehavioralReceiptError {
    message: String,
}

impl BehavioralReceiptError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BehavioralReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BehavioralReceiptError {}

pub fn validate_receipt(receipt: &BehavioralReceipt) -> Result<(), BehavioralReceiptError> {
    if receipt.schema_version != BEHAVIORAL_RECEIPT_SCHEMA_VERSION {
        return Err(BehavioralReceiptError::new(format!(
            "unsupported behavioral receipt schema {}; expected {BEHAVIORAL_RECEIPT_SCHEMA_VERSION}",
            receipt.schema_version
        )));
    }

    validate_coordinate(&receipt.repository, "repository")?;
    validate_git_revision(&receipt.revision)?;
    validate_coordinate(&receipt.task, "task")?;
    validate_coordinate(&receipt.evidence_kind, "evidence_kind")?;
    validate_coordinate(&receipt.evidence_ref, "evidence_ref")?;
    validate_action(receipt.action.as_deref())?;

    match receipt.delivery {
        Delivery::Quiet => validate_quiet(receipt),
        Delivery::Surfaced => validate_surfaced(receipt),
    }
}

fn validate_quiet(receipt: &BehavioralReceipt) -> Result<(), BehavioralReceiptError> {
    if receipt.consulted {
        return Err(BehavioralReceiptError::new(
            "quiet evidence cannot be marked consulted",
        ));
    }
    if receipt.outcome != BehavioralOutcome::CorrectQuietNegative {
        return Err(BehavioralReceiptError::new(
            "quiet delivery requires outcome=correct_quiet_negative",
        ));
    }
    if receipt.action.is_some() {
        return Err(BehavioralReceiptError::new(
            "correct quiet negatives cannot record a changed next action",
        ));
    }
    Ok(())
}

fn validate_surfaced(receipt: &BehavioralReceipt) -> Result<(), BehavioralReceiptError> {
    if receipt.outcome == BehavioralOutcome::CorrectQuietNegative {
        return Err(BehavioralReceiptError::new(
            "surfaced evidence cannot use outcome=correct_quiet_negative",
        ));
    }

    if receipt.outcome == BehavioralOutcome::Ignored {
        if receipt.consulted {
            return Err(BehavioralReceiptError::new(
                "ignored evidence cannot be marked consulted",
            ));
        }
        if receipt.action.is_some() {
            return Err(BehavioralReceiptError::new(
                "ignored evidence cannot record a changed next action",
            ));
        }
        return Ok(());
    }

    if !receipt.consulted {
        return Err(BehavioralReceiptError::new(format!(
            "outcome={} requires consulted=true",
            outcome_name(receipt.outcome)
        )));
    }

    match receipt.outcome {
        BehavioralOutcome::ChangedNextAction
        | BehavioralOutcome::PreventedOrReversedWrongTurn
        | BehavioralOutcome::StaleOrWrongCoordinate
        | BehavioralOutcome::NeededStrongerEvidence => {
            if receipt.action.is_none() {
                return Err(BehavioralReceiptError::new(format!(
                    "outcome={} requires a concrete action",
                    outcome_name(receipt.outcome)
                )));
            }
        }
        BehavioralOutcome::UsefulSameAction | BehavioralOutcome::Irrelevant => {
            if receipt.action.is_some() {
                return Err(BehavioralReceiptError::new(format!(
                    "outcome={} must omit action because the next action did not change",
                    outcome_name(receipt.outcome)
                )));
            }
        }
        BehavioralOutcome::Ignored | BehavioralOutcome::CorrectQuietNegative => unreachable!(),
    }

    Ok(())
}

fn validate_coordinate(value: &str, field: &str) -> Result<(), BehavioralReceiptError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_COORDINATE_BYTES
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(BehavioralReceiptError::new(format!(
            "{field} must be a bounded non-empty canonical coordinate"
        )));
    }
    Ok(())
}

fn validate_git_revision(revision: &str) -> Result<(), BehavioralReceiptError> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BehavioralReceiptError::new(
            "revision must be an exact 40-character lowercase Git object id",
        ));
    }
    Ok(())
}

fn validate_action(action: Option<&str>) -> Result<(), BehavioralReceiptError> {
    let Some(action) = action else {
        return Ok(());
    };
    if action.is_empty()
        || action.trim() != action
        || action.len() > MAX_ACTION_BYTES
        || action.contains('\0')
        || action.chars().any(char::is_control)
    {
        return Err(BehavioralReceiptError::new(
            "action must be bounded, non-empty, and single-line when present",
        ));
    }
    Ok(())
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
