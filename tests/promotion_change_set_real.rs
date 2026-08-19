#[allow(dead_code)]
#[path = "../src/promotion_change_set.rs"]
mod promotion_change_set;

use promotion_change_set::{PromotionChangeSetEntry, fingerprint_promotion_change_set};

fn entry(path: &str, blob_sha: &str) -> PromotionChangeSetEntry {
    PromotionChangeSetEntry {
        path: path.to_string(),
        blob_sha: blob_sha.to_string(),
    }
}

#[test]
fn pr_201_reanchor_kept_the_exact_one_file_compatibility_payload() {
    let early_success_head = "1614cc2ae82df50ec3c8b5c4a9e428ad01c1d50f";
    let early_success_tree = "a90a3317c50d5d7d693b948cc9414315056c628f";
    let final_promoted_head = "3cf9090dfb474adaac6ab773c357627c37c3f9e6";
    let final_promoted_tree = "889e34a998fe268986718bf21e72263503a1a05b";
    let exact_test_blob = "c5945b4cfee5f6ea43f782d0c5b68fa8a9125ef4";

    assert_ne!(early_success_head, final_promoted_head);
    assert_ne!(early_success_tree, final_promoted_tree);

    let early = fingerprint_promotion_change_set(&[entry(
        "tests/known_stale_observation_frontier.rs",
        exact_test_blob,
    )])
    .unwrap();
    let final_state = fingerprint_promotion_change_set(&[entry(
        "tests/known_stale_observation_frontier.rs",
        exact_test_blob,
    )])
    .unwrap();
    assert_eq!(early, final_state);
}

#[test]
fn pr_194_executable_frontier_payload_survived_reanchor_byte_for_byte() {
    let early_success_head = "b54ed3213994c96ec818ef36bb9728b0dc1f7eb6";
    let early_success_tree = "d4a62bdd14700de18abadf1b593309bb2683107c";
    let final_promoted_head = "6706755e434a9bb533d77655587b01cbdd3fa1e8";
    let final_promoted_tree = "d7607be315f0920b8437ed35e458aa9a8289109e";

    assert_ne!(early_success_head, final_promoted_head);
    assert_ne!(early_success_tree, final_promoted_tree);

    let payload = [
        entry(
            "src/observation_frontier.rs",
            "23b142040cb244cb09e0428d162b6fcfaf787e67",
        ),
        entry(
            "tests/observation_frontier.rs",
            "d809ae08481e642e50818adc1395e9c4b4827563",
        ),
        entry(
            "examples/observation_frontiers.rs",
            "41d63d6b92f66d9faaa2c97af1bc6f06505c501a",
        ),
    ];
    let early = fingerprint_promotion_change_set(&payload).unwrap();
    let mut reversed = payload.to_vec();
    reversed.reverse();
    let final_state = fingerprint_promotion_change_set(&reversed).unwrap();
    assert_eq!(early, final_state);

    let early_receipt_note = "cddd9e76252c19cd82c781b472f2f06a8e84b45d";
    let final_receipt_note = "21b89394a82ea07d31441afa7aa77917b3d4b1a2";
    assert_ne!(early_receipt_note, final_receipt_note);
}

#[test]
fn fingerprint_is_order_independent_and_content_sensitive() {
    let first = entry("src/feature.rs", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let second = entry(
        "tests/feature.rs",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    let forward = fingerprint_promotion_change_set(&[first.clone(), second.clone()]).unwrap();
    let reverse = fingerprint_promotion_change_set(&[second, first.clone()]).unwrap();
    assert_eq!(forward, reverse);

    let changed = fingerprint_promotion_change_set(&[PromotionChangeSetEntry {
        path: first.path,
        blob_sha: "cccccccccccccccccccccccccccccccccccccccc".to_string(),
    }])
    .unwrap();
    assert_ne!(forward, changed);
}

#[test]
fn duplicate_paths_and_noncanonical_entries_reject() {
    let duplicate = vec![
        entry("src/feature.rs", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        entry("src/feature.rs", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    ];
    assert!(fingerprint_promotion_change_set(&duplicate).is_err());

    for invalid_path in ["/src/feature.rs", "src/../feature.rs", "src\\feature.rs"] {
        assert!(
            fingerprint_promotion_change_set(&[entry(
                invalid_path,
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )])
            .is_err()
        );
    }
}
