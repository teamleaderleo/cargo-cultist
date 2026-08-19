#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/review_memory.rs"]
mod review_memory;

use applicability::EvaluationContext;
use review_memory::{
    CurrentConcern, ReviewMemoryQuery, ReviewThreadDisposition, REVIEW_MEMORY_SCHEMA_VERSION,
    evaluate_review_memory,
};

const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn query(context: EvaluationContext) -> ReviewMemoryQuery {
    ReviewMemoryQuery {
        schema_version: REVIEW_MEMORY_SCHEMA_VERSION,
        current: CurrentConcern {
            concern_key: "review:fixture".to_string(),
            context,
        },
        records: Vec::new(),
    }
}

#[test]
fn empty_memory_with_missing_head_requires_context() {
    let evaluation = evaluate_review_memory(&query(EvaluationContext {
        repository: Some("owner/repo".to_string()),
        revision: None,
        work: Some("#7".to_string()),
        path: None,
    }))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NeedContext);
    assert!(evaluation.matches.is_empty());
}

#[test]
fn empty_memory_with_missing_repository_requires_context() {
    let evaluation = evaluate_review_memory(&query(EvaluationContext {
        repository: None,
        revision: Some(HEAD.to_string()),
        work: Some("#7".to_string()),
        path: None,
    }))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NeedContext);
}

#[test]
fn empty_memory_with_missing_work_requires_context() {
    let evaluation = evaluate_review_memory(&query(EvaluationContext {
        repository: Some("owner/repo".to_string()),
        revision: Some(HEAD.to_string()),
        work: None,
        path: None,
    }))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NeedContext);
}
