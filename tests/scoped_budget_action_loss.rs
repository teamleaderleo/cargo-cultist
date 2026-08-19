#![allow(dead_code)]
#![allow(clippy::field_reassign_with_default)]

mod packet {
    include!("../examples/agent_context_packet.rs");
    include!("../examples/support/scoped_agent_context_packet_impl.rs");

    #[cfg(test)]
    mod adversarial_budget {
        use super::*;

        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        enum NextAction {
            InspectDistributedFailureFamily,
            ContinueFileLocal,
        }

        fn summary(sha: &str, subject: &str) -> CommitSummary {
            CommitSummary {
                sha: sha.to_string(),
                date: "2026-08-19T00:00:00Z".to_string(),
                subject: subject.to_string(),
            }
        }

        fn file_packet(max_serialized_bytes: usize) -> AgentContextPacket {
            let budget = PacketBudget {
                max_serialized_bytes,
                ..PacketBudget::default()
            };
            AgentContextPacket {
                schema_version: SCHEMA_VERSION,
                analysis: "agent_context",
                repository: "/repo".to_string(),
                target: PacketTarget {
                    path: "pkg/target.rs".to_string(),
                },
                budget,
                candidate_serialized_bytes: 0,
                serialized_bytes: 0,
                semantic_evictions: Vec::new(),
                direct_evidence: vec![EvidenceItem {
                    claim_kind: "proven",
                    message: "target identity".to_string(),
                    source: EvidenceSource {
                        kind: "filesystem",
                        path: "pkg/target.rs".to_string(),
                    },
                }],
                guidance: Vec::new(),
                reviewed_decisions: Vec::new(),
                recent_history: vec![summary(
                    "target-local",
                    "routine target-local context that does not expose the distributed failure family",
                )],
                historical_companions: Vec::new(),
                companion_exclusions: Vec::new(),
                unknowns: vec!["distributed failure relevance is unresolved".to_string()],
                truncation: Vec::new(),
            }
        }

        fn modeled_next_action(envelope: &ScopedAgentContextEnvelope) -> NextAction {
            let has_a = envelope
                .scope_recent_history
                .iter()
                .any(|commit| commit.subject.contains("DISTRIBUTED_LESSON_A"));
            let has_b = envelope
                .scope_recent_history
                .iter()
                .any(|commit| commit.subject.contains("DISTRIBUTED_LESSON_B"));
            if has_a && has_b {
                NextAction::InspectDistributedFailureFamily
            } else {
                NextAction::ContinueFileLocal
            }
        }

        #[test]
        fn scope_first_eviction_can_change_modeled_action_while_target_history_survives() {
            let mut file_packet = file_packet(usize::MAX);
            stabilize_candidate_size(&mut file_packet).unwrap();
            let mut envelope = ScopedAgentContextEnvelope {
                schema_version: SCOPED_SCHEMA_VERSION,
                analysis: "scoped_agent_context",
                repository: "/repo".to_string(),
                target: "pkg/target.rs".to_string(),
                scope: "pkg".to_string(),
                budget: ScopedEnvelopeBudget {
                    max_serialized_bytes: usize::MAX,
                    max_scope_history_commits: DEFAULT_MAX_SCOPE_HISTORY,
                },
                candidate_serialized_bytes: 0,
                serialized_bytes: 0,
                semantic_evictions: Vec::new(),
                file_packet,
                scope_recent_history: vec![
                    summary(
                        "scope-a",
                        &format!("DISTRIBUTED_LESSON_A {}", "a".repeat(700)),
                    ),
                    summary(
                        "scope-b",
                        &format!("DISTRIBUTED_LESSON_B {}", "b".repeat(700)),
                    ),
                ],
                scope_history_truncated: true,
                unknowns: vec!["scope chronology is not semantic proof".to_string()],
            };

            stabilize_scoped_candidate_size(&mut envelope).unwrap();
            let candidate = envelope.candidate_serialized_bytes;
            let target_history_before = envelope.file_packet.recent_history.len();
            assert_eq!(
                modeled_next_action(&envelope),
                NextAction::InspectDistributedFailureFamily
            );

            envelope.budget.max_serialized_bytes = candidate - 200;
            envelope.file_packet.budget.max_serialized_bytes = candidate - 200;
            let rendered = compile_scoped_to_budget(&mut envelope).unwrap();

            assert!(rendered.len() <= envelope.budget.max_serialized_bytes);
            assert_eq!(envelope.scope_recent_history.len(), 1);
            assert_eq!(
                envelope.file_packet.recent_history.len(),
                target_history_before
            );
            assert_eq!(
                envelope.semantic_evictions.first(),
                Some(&SemanticEviction {
                    class: "scope_recent_history_summary",
                    count: 1,
                })
            );
            assert_eq!(
                modeled_next_action(&envelope),
                NextAction::ContinueFileLocal
            );
        }
    }
}
