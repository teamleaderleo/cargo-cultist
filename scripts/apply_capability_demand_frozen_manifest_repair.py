#!/usr/bin/env python3
"""Apply the already-verified semantic repair embedded in the temporary carrier.

This helper deliberately restores the real materializer workflow after applying the
embedded patch. GitHub Actions can then push the non-workflow semantic files; the
materializer workflow is updated separately through an authorized repository write.
"""

from __future__ import annotations

import subprocess
import textwrap
from pathlib import Path

CARRIER = Path(".github/workflows/repair-capability-demand-frozen-manifest-binding.yml")
MATERIALIZER = Path(".github/workflows/capability-demand-retirement-inputs.yml")
EXPECTED_CHANGED = {
    "research/capability-demand-retirement/run-receipts.md",
    "research/capability-demand-retirement/stensibly-convex-index-review-v1-input-manifest-32264661913.json",
    "src/capability_demand_retirement.rs",
    "tests/capability_demand_retirement_receipts.rs",
}


def main() -> int:
    lines = CARRIER.read_text(encoding="utf-8").splitlines()
    start = lines.index("          python - <<'PY'") + 1
    end = lines.index("          PY", start)
    source = textwrap.dedent("\n".join(lines[start:end])) + "\n"
    namespace = {"__name__": "__embedded_frozen_manifest_repair__"}
    exec(compile(source, str(CARRIER), "exec"), namespace)

    subprocess.run(
        ["git", "restore", "--", str(MATERIALIZER)],
        check=True,
    )

    completed = subprocess.run(
        ["git", "diff", "--name-only"],
        check=True,
        capture_output=True,
        text=True,
    )
    changed = {line for line in completed.stdout.splitlines() if line}
    if changed != EXPECTED_CHANGED:
        raise SystemExit(
            "unexpected semantic repair path set: "
            f"expected {sorted(EXPECTED_CHANGED)}, got {sorted(changed)}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
