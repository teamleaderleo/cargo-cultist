#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct FindingKey {
    kind: &'static str,
    target: &'static str,
}

impl FindingKey {
    fn new(kind: &'static str, target: &'static str) -> Self {
        Self { kind, target }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct FindingState {
    key: FindingKey,
    message: &'static str,
    resolved: bool,
}

impl FindingState {
    fn new(key: FindingKey, message: &'static str) -> Self {
        Self {
            key,
            message,
            resolved: false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Snapshot {
    findings: Vec<FindingState>,
}

impl Snapshot {
    fn new(findings: Vec<FindingState>) -> Self {
        Self { findings }
    }

    fn fingerprint(&self) -> u64 {
        // Research-only deterministic FNV-1a over the exact ordered snapshot.
        // Production identity needs an explicit canonical codec/hash contract;
        // this fixture only proves that exact-base binding changes semantics.
        let mut hash = 0xcbf29ce484222325u64;
        for finding in &self.findings {
            for byte in finding.key.kind.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(0x100000001b3);
            for byte in finding.key.target.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash ^= 0xfe;
            hash = hash.wrapping_mul(0x100000001b3);
            for byte in finding.message.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash ^= u64::from(finding.resolved);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PositionalDelta {
    base: u64,
    index: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct KeyedDelta {
    base: Option<u64>,
    key: FindingKey,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ApplyError {
    BaseMismatch,
    MissingTarget,
}

fn apply_position_unchecked(snapshot: &mut Snapshot, delta: &PositionalDelta) -> Result<(), ApplyError> {
    let finding = snapshot
        .findings
        .get_mut(delta.index)
        .ok_or(ApplyError::MissingTarget)?;
    finding.resolved = true;
    Ok(())
}

fn apply_position_checked(snapshot: &mut Snapshot, delta: &PositionalDelta) -> Result<(), ApplyError> {
    if snapshot.fingerprint() != delta.base {
        return Err(ApplyError::BaseMismatch);
    }
    apply_position_unchecked(snapshot, delta)
}

fn apply_keyed(snapshot: &mut Snapshot, delta: &KeyedDelta) -> Result<(), ApplyError> {
    if let Some(base) = delta.base
        && snapshot.fingerprint() != base
    {
        return Err(ApplyError::BaseMismatch);
    }

    let finding = snapshot
        .findings
        .iter_mut()
        .find(|finding| finding.key == delta.key)
        .ok_or(ApplyError::MissingTarget)?;
    finding.resolved = true;
    Ok(())
}

fn auth_finding() -> FindingState {
    FindingState::new(
        FindingKey::new("preflight-overlap", "src/auth.rs"),
        "active work overlaps the authorization path",
    )
}

fn cache_finding() -> FindingState {
    FindingState::new(
        FindingKey::new("generated-companion", "src/cache.rs"),
        "generated companion evidence is incomplete",
    )
}

#[test]
fn positional_ref_changes_meaning_after_pure_reorder() {
    let original = Snapshot::new(vec![auth_finding(), cache_finding()]);
    let reordered = Snapshot::new(vec![cache_finding(), auth_finding()]);
    let delta = PositionalDelta {
        base: original.fingerprint(),
        index: 0,
    };

    let mut wrong_base = reordered.clone();
    apply_position_unchecked(&mut wrong_base, &delta).unwrap();

    assert!(wrong_base.findings[0].resolved);
    assert_eq!(wrong_base.findings[0].key, cache_finding().key);
    assert!(!wrong_base.findings[1].resolved);
    assert_eq!(wrong_base.findings[1].key, auth_finding().key);
}

#[test]
fn exact_base_binding_rejects_the_same_positional_delta_after_reorder() {
    let original = Snapshot::new(vec![auth_finding(), cache_finding()]);
    let reordered = Snapshot::new(vec![cache_finding(), auth_finding()]);
    let delta = PositionalDelta {
        base: original.fingerprint(),
        index: 0,
    };

    let mut receiver = reordered;
    assert_eq!(
        apply_position_checked(&mut receiver, &delta),
        Err(ApplyError::BaseMismatch)
    );
    assert!(receiver.findings.iter().all(|finding| !finding.resolved));
}

#[test]
fn keyed_identity_survives_pure_reorder_when_the_receiver_explicitly_allows_rebase() {
    let original = Snapshot::new(vec![auth_finding(), cache_finding()]);
    let mut reordered = Snapshot::new(vec![cache_finding(), auth_finding()]);
    let delta = KeyedDelta {
        // None models an explicitly chosen semantic-key lookup rather than an
        // accidental stale positional application. A real protocol should be
        // stricter by default and require an explicit rebase/lookup mode.
        base: None,
        key: original.findings[0].key.clone(),
    };

    apply_keyed(&mut reordered, &delta).unwrap();

    let auth = reordered
        .findings
        .iter()
        .find(|finding| finding.key == auth_finding().key)
        .unwrap();
    let cache = reordered
        .findings
        .iter()
        .find(|finding| finding.key == cache_finding().key)
        .unwrap();
    assert!(auth.resolved);
    assert!(!cache.resolved);
}

#[test]
fn changing_the_semantic_target_changes_the_synthetic_identity() {
    let old = FindingKey::new("preflight-overlap", "src/auth.rs");
    let moved = FindingKey::new("preflight-overlap", "src/authorization.rs");
    let different_kind = FindingKey::new("policy-collision", "src/auth.rs");

    assert_ne!(old, moved);
    assert_ne!(old, different_kind);
}

#[test]
fn snapshot_fingerprint_changes_on_order_and_resolution_state() {
    let original = Snapshot::new(vec![auth_finding(), cache_finding()]);
    let reordered = Snapshot::new(vec![cache_finding(), auth_finding()]);
    assert_ne!(original.fingerprint(), reordered.fingerprint());

    let mut resolved = original.clone();
    resolved.findings[0].resolved = true;
    assert_ne!(original.fingerprint(), resolved.fingerprint());
}
