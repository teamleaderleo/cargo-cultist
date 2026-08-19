use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq)]
struct SkillReferenceContext {
    target_repository_root: PathBuf,
    skill_root: PathBuf,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ReferenceResolution<'a> {
    AmbientTarget(&'a Path),
    ExplicitSkillRoot(&'a Path),
}

fn validate_relative_reference(reference: &Path) -> bool {
    !reference.as_os_str().is_empty()
        && !reference.is_absolute()
        && reference
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn resolve_reference(
    context: &SkillReferenceContext,
    reference: &Path,
    mode: ReferenceResolution<'_>,
) -> Option<PathBuf> {
    if !validate_relative_reference(reference) {
        return None;
    }
    let root = match mode {
        ReferenceResolution::AmbientTarget(root) => root,
        ReferenceResolution::ExplicitSkillRoot(root) => root,
    };
    Some(root.join(reference))
}

fn fixture_context(target_root: &str) -> SkillReferenceContext {
    SkillReferenceContext {
        target_repository_root: PathBuf::from(target_root),
        skill_root: PathBuf::from("/installed-skills/gh-address-comments"),
    }
}

fn bundled_skill_paths() -> BTreeSet<PathBuf> {
    [PathBuf::from(
        "/installed-skills/gh-address-comments/scripts/fetch_comments.py",
    )]
    .into_iter()
    .collect()
}

#[test]
fn same_relative_reference_resolves_to_different_objects_under_two_ambient_roots() {
    let context = fixture_context("/workspace/repository-a");
    let reference = Path::new("scripts/fetch_comments.py");

    let target_resolution = resolve_reference(
        &context,
        reference,
        ReferenceResolution::AmbientTarget(&context.target_repository_root),
    )
    .unwrap();
    let skill_resolution = resolve_reference(
        &context,
        reference,
        ReferenceResolution::ExplicitSkillRoot(&context.skill_root),
    )
    .unwrap();

    assert_eq!(
        target_resolution,
        PathBuf::from("/workspace/repository-a/scripts/fetch_comments.py")
    );
    assert_eq!(
        skill_resolution,
        PathBuf::from("/installed-skills/gh-address-comments/scripts/fetch_comments.py")
    );
    assert_ne!(target_resolution, skill_resolution);
}

#[test]
fn known_bundled_helper_is_recoverable_from_explicit_skill_root_but_not_target_cwd() {
    let context = fixture_context("/workspace/repository-a");
    let reference = Path::new("scripts/fetch_comments.py");
    let known = bundled_skill_paths();

    let target_resolution = resolve_reference(
        &context,
        reference,
        ReferenceResolution::AmbientTarget(&context.target_repository_root),
    )
    .unwrap();
    let skill_resolution = resolve_reference(
        &context,
        reference,
        ReferenceResolution::ExplicitSkillRoot(&context.skill_root),
    )
    .unwrap();

    assert!(!known.contains(&target_resolution));
    assert!(known.contains(&skill_resolution));
}

#[test]
fn explicit_skill_root_resolution_is_invariant_to_target_repository_movement() {
    let first = fixture_context("/workspace/repository-a");
    let second = fixture_context("/tmp/another-checkout");
    let reference = Path::new("scripts/fetch_comments.py");

    let first_target = resolve_reference(
        &first,
        reference,
        ReferenceResolution::AmbientTarget(&first.target_repository_root),
    )
    .unwrap();
    let second_target = resolve_reference(
        &second,
        reference,
        ReferenceResolution::AmbientTarget(&second.target_repository_root),
    )
    .unwrap();
    let first_bound = resolve_reference(
        &first,
        reference,
        ReferenceResolution::ExplicitSkillRoot(&first.skill_root),
    )
    .unwrap();
    let second_bound = resolve_reference(
        &second,
        reference,
        ReferenceResolution::ExplicitSkillRoot(&second.skill_root),
    )
    .unwrap();

    assert_ne!(first_target, second_target);
    assert_eq!(first_bound, second_bound);
}

#[test]
fn explicit_root_binding_does_not_make_traversal_or_absolute_references_valid() {
    let context = fixture_context("/workspace/repository-a");

    for invalid in ["../scripts/fetch_comments.py", "/tmp/fetch_comments.py", ""] {
        let reference = Path::new(invalid);
        assert!(
            resolve_reference(
                &context,
                reference,
                ReferenceResolution::ExplicitSkillRoot(&context.skill_root),
            )
            .is_none()
        );
    }
}
