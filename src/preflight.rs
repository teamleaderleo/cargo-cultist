use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

pub fn build_preflight_analysis_report(
    root: &Path,
    against: &str,
    scope: Option<&Path>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let anchor = merge_base(root, against)?;
    let current_paths = changed_paths(root, &anchor, None, scope)?;
    let other_paths = changed_paths(root, &anchor, Some(against), scope)?;

    let mut analysis =
        AnalysisReport::new("preflight-collisions", root.to_string_lossy().into_owned());
    analysis.claims.push(Claim::new(
        ClaimKind::Derived,
        format!("Preflight compares current work and `{against}` from merge base `{anchor}`."),
    ));
    analysis.claims.push(Claim::new(
        ClaimKind::Proven,
        format!(
            "Current work modifies {} path(s); `{against}` modifies {} path(s) in the selected scope.",
            current_paths.len(),
            other_paths.len()
        ),
    ));

    for path in current_paths.intersection(&other_paths) {
        let display = path.to_string_lossy().into_owned();
        analysis.findings.push(
            Finding::new("preflight-direct-path-overlap", "Concurrent path overlap")
                .at(Location::new(display.clone(), None))
                .with_claim(
                    Claim::new(
                        ClaimKind::Proven,
                        format!("Both current work and `{against}` modify `{display}`."),
                    )
                    .with_evidence(Evidence::at(
                        format!("Current work changes `{display}` from merge base `{anchor}`."),
                        Location::new(display.clone(), None),
                    ))
                    .with_evidence(Evidence::at(
                        format!("`{against}` changes `{display}` from the same merge base."),
                        Location::new(display.clone(), None),
                    )),
                )
                .with_question(
                    "Should ownership, ordering, or intent be coordinated before these changes proceed independently?",
                ),
        );
    }

    if analysis.findings.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Proven,
            "The two change sets have no direct repository-path overlap in the selected scope.",
        ));
    }

    analysis.claims.push(Claim::new(
        ClaimKind::Unknown,
        "Direct path comparison does not determine whether different paths participate in the same generated, historical, policy, or behavioral relationship.",
    ));

    Ok(analysis)
}

fn changed_paths(
    root: &Path,
    anchor: &str,
    target: Option<&str>,
    scope: Option<&Path>,
) -> Result<BTreeSet<PathBuf>, Box<dyn Error>> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args([
        "-c",
        "core.quotepath=false",
        "diff",
        "--name-only",
        "--no-renames",
        "-z",
        anchor,
    ]);

    if let Some(target) = target {
        command.arg(target);
    }

    command.arg("--");
    if let Some(scope) = scope {
        command.arg(scope);
    }

    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed while building preflight evidence: {stderr}").into());
    }

    let mut paths = BTreeSet::new();
    for raw in output.stdout.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw)?;
        paths.insert(PathBuf::from(path));
    }
    Ok(paths)
}

fn merge_base(root: &Path, against: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", against, "HEAD"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("could not find merge base for `{against}` and HEAD: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-cultist-preflight-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git command failed: git {args:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_repo(name: &str) -> PathBuf {
        let root = unique_temp_dir(name);
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "-q", "-b", "main"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);
        fs::write(root.join("shared.txt"), "baseline\n").unwrap();
        fs::write(root.join("current.txt"), "baseline\n").unwrap();
        fs::write(root.join("other.txt"), "baseline\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);
        root
    }

    fn make_other_branch(root: &Path, path: &str, contents: &str) {
        run_git(root, &["switch", "-q", "-c", "other"]);
        fs::write(root.join(path), contents).unwrap();
        run_git(root, &["add", path]);
        run_git(root, &["commit", "-q", "-m", "other change"]);
        run_git(root, &["switch", "-q", "main"]);
    }

    #[test]
    fn reports_direct_path_overlap() {
        let root = init_repo("direct-overlap");
        make_other_branch(&root, "shared.txt", "other\n");
        fs::write(root.join("shared.txt"), "current\n").unwrap();

        let analysis = build_preflight_analysis_report(&root, "other", None).unwrap();

        assert_eq!(analysis.findings.len(), 1);
        assert_eq!(analysis.findings[0].kind, "preflight-direct-path-overlap");
        assert_eq!(
            analysis.findings[0].location.as_ref().unwrap().path,
            "shared.txt"
        );
        assert_eq!(analysis.findings[0].claims[0].kind, ClaimKind::Proven);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disjoint_paths_remain_explicitly_unknown_semantically() {
        let root = init_repo("disjoint");
        make_other_branch(&root, "other.txt", "other\n");
        fs::write(root.join("current.txt"), "current\n").unwrap();

        let analysis = build_preflight_analysis_report(&root, "other", None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Proven
                && claim.message.contains("no direct repository-path overlap")
        }));
        assert!(
            analysis
                .claims
                .iter()
                .any(|claim| claim.kind == ClaimKind::Unknown)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn includes_staged_and_unstaged_current_changes() {
        let root = init_repo("staged-and-unstaged");
        make_other_branch(&root, "shared.txt", "other shared\n");

        fs::write(root.join("shared.txt"), "staged shared\n").unwrap();
        run_git(&root, &["add", "shared.txt"]);
        fs::write(root.join("current.txt"), "unstaged current\n").unwrap();

        let analysis = build_preflight_analysis_report(&root, "other", None).unwrap();

        assert_eq!(analysis.findings.len(), 1);
        assert!(analysis.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Proven
                && claim.message.contains("Current work modifies 2 path(s)")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn branch_commits_on_current_side_are_measured_from_merge_base() {
        let root = init_repo("merge-base");
        make_other_branch(&root, "shared.txt", "other\n");

        fs::write(root.join("shared.txt"), "current committed\n").unwrap();
        run_git(&root, &["add", "shared.txt"]);
        run_git(&root, &["commit", "-q", "-m", "current change"]);

        let analysis = build_preflight_analysis_report(&root, "other", None).unwrap();

        assert_eq!(analysis.findings.len(), 1);
        assert_eq!(
            analysis.findings[0].location.as_ref().unwrap().path,
            "shared.txt"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
