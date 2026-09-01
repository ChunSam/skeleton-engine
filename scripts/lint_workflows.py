#!/usr/bin/env python3
"""Fails on workflow-expression mistakes GitHub only reports after a merge.

    ./scripts/lint_workflows.py [path ...]      # default: .github/workflows/*.yml

── Why this exists ─────────────────────────────────────────────────────────────────────────────

A workflow file is parsed by GitHub, not by us, and a file it cannot parse produces a run that
fails before any step — visible only once the file is on the default branch, because a
`schedule` / `workflow_dispatch` workflow never runs on a pull request. `.github/workflows/soak.yml`
shipped that way on 2026-09-01 (#528) and `gh workflow run` answered:

    HTTP 422: failed to parse workflow: (Line: 95, Col: 14): An expression was expected

⚠️ The cause is worth more than the fix. The offending text was inside a `#` comment in a `run:`
block, explaining that an expression is substituted before the shell sees it — and it was, because
**the expression parser does not know what a shell comment is.** `python3 -c 'yaml.safe_load(...)'`
had validated the file happily: it is valid YAML and invalid GitHub Actions, and only the second
one matters.

── What it checks, and what it deliberately does not ───────────────────────────────────────────

This is not actionlint. It catches the specific class above — an expression GitHub will refuse —
and nothing else:

  * `${{ }}` with an empty or whitespace-only body   -> "An expression was expected"
  * `${{` with no closing `}}` on the same line      -> an unterminated expression

Everything else (context names, step ids, shellcheck) is out of scope on purpose: a linter that
half-checks a thing invites the belief that it fully checks it. If this file grows a third rule,
adopt actionlint instead of extending this.
"""

import glob
import re
import sys

EXPR = re.compile(r"\$\{\{(.*?)\}\}")
OPEN = "${{"


def lint(path):
    """Returns a list of `path:line: message` problems."""
    problems = []
    with open(path, encoding="utf-8") as fh:
        for lineno, line in enumerate(fh, 1):
            closed = 0
            for match in EXPR.finditer(line):
                closed += 1
                if not match.group(1).strip():
                    problems.append(
                        f"{path}:{lineno}: empty workflow expression — GitHub answers "
                        f'"An expression was expected" and refuses the whole file. If you are '
                        f"writing ABOUT the syntax, say so in words: a comment is not a comment "
                        f"to the expression parser."
                    )
            # An opening with no partner on the same line. Expressions do span lines in YAML block
            # scalars, but not in this repo's workflows, and the false positive is cheap to fix
            # while the failure it catches costs a merge.
            if line.count(OPEN) > closed:
                problems.append(
                    f"{path}:{lineno}: unterminated workflow expression — "
                    f"{line.count(OPEN)} '{OPEN}' against {closed} closed."
                )
    return problems


def main(argv):
    paths = argv[1:] or sorted(
        glob.glob(".github/workflows/*.yml") + glob.glob(".github/workflows/*.yaml")
    )
    if not paths:
        print("[lint-workflows] no workflow files found — nothing checked, which proves nothing.")
        return 0

    problems = []
    for path in paths:
        problems.extend(lint(path))

    for problem in problems:
        print(f"[lint-workflows] {problem}", file=sys.stderr)

    if problems:
        print(
            f"[lint-workflows] FAIL: {len(problems)} problem(s) in {len(paths)} file(s)",
            file=sys.stderr,
        )
        return 1

    print(f"[lint-workflows] {len(paths)} workflow file(s) OK ✓")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
