# Codex project instructions

Read `CLAUDE.md` before working in this repository. It is the shared source of
engine conventions, architecture constraints, verification scope, and Git rules.
Also read `docs/AGENT_WORKFLOW.md` for task scoping and reporting. Follow their
links selectively; do not load the entire documentation tree.

## Applying the shared rules

- Follow explicit user scope and authorization. Do not ask again for an action
  the user already authorized. Prepare reviewable work before any needed approval.
- Apply the scoped Verification rules in `CLAUDE.md`, not the unqualified opening
  sentence alone. Documentation/configuration-only work needs relevant structural
  checks; engine changes need the existing gate. Report checks actually run.
- Delegate only when authorized by the active task/environment and useful for a
  concrete independent subtask. Shared recommendations do not override that limit.
- Keep responses concise, in the user's language. Report results, evidence, and
  remaining limitations; do not claim a build proves browser or device behavior.
- Never stage, commit, or push unless explicitly requested.

## Codex setup

`.codex/config.toml` selects GPT-6 Astra with medium reasoning for this project.
Existing task selections and higher-priority settings can override this default.
Repository skills live in `.agents/skills/`; use their references as needed.
They were ported from local `.claude/skills/`. Maintain the Codex copies here;
the ignored Claude copies remain available for the previous workflow.

See `docs/CODEX_SETUP.md` for verification, limitations, and rollback.
