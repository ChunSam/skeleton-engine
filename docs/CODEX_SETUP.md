# Codex / GPT-6 Astra setup

The engine has no discovered OpenAI model calls to migrate. This setup changes
the coding environment: `.codex/config.toml` selects `gpt-6-astra` with `medium`
reasoning, and root `AGENTS.md` routes to the existing shared instructions.
No Rust dependencies, runtime APIs, or release version change.

## Instructions and skills

`CLAUDE.md` and `docs/AGENT_WORKFLOW.md` remain the shared rule sources.
`AGENTS.md` clarifies verification scope and existing user authorization without
duplicating engine rules. Codex skills and their references are checked-in-ready
copies under `.agents/skills/`: alloc-measure, browser-smoke, example-selftest,
prose-sweep, sabotage-check, session-cleanup, and subsystem-review.
Automatic discovery remains enabled; cleanup still requires a cleanup request
and does not expand its scope to unrelated caches.

The local `.claude/skills/` originals are preserved and ignored by Git. Changes
to Codex skills are maintained in `.agents/skills/`; copies do not synchronize.
Historical notes mention Claude hooks, but no hook settings/scripts were found
in the current project inventory. No equivalent automatic hook is claimed.
The verification scripts and CI remain the enforcement mechanisms.

## Validate a new session

Start a fresh task in this trusted project. Project configuration can be skipped
for untrusted projects; explicit task/model settings can override the default.
Check the actual selected model in the task UI. Ask the agent to list its loaded
instructions and available repository skills, and classify these three cases:

| Case | Expected behavior |
| --- | --- |
| Documentation-only edit | Relevant links/structure checks; explain skipped engine gate |
| ECS logic change | Related regression test and `scripts/verify.sh` |
| Rendering/WASM change | Engine gate plus relevant GPU/browser runtime checks |

Use actual requested development tasks for quality/time comparisons against the
previous environment. Do not invent engine changes solely to exercise migration.
Static configuration checks are not proof of model access, automatic discovery,
or improved quality. Those require a fresh live session and representative work.

## Rollback

Restore the prior model selection or remove only the two model settings added in
`.codex/config.toml`. The original Claude workflow remains present. If reverting
the integration files, review the diff first and preserve subsequent user edits.
No dependency or engine-code rollback is needed.

## Validation record (2026-09-11)

- All seven skills pass the bundled Skill Creator validator. Three descriptions
  were normalized to remove angle-bracket placeholders; referenced files exist.
- `cargo check --locked` and `git diff --check` pass. `CLAUDE.md` remains 200 lines.
- Before the update, CLI 0.134.0 (`/opt/homebrew/bin/codex`) loaded the project model
  `gpt-6-astra` and reasoning `medium`. Its live request fails with HTTP 400:
  the server requires a newer Codex. It also cannot decode the current model
  catalog's `max` reasoning level.
- After explicit user approval, the app-bundled CLI 0.153.4 at
  `/Applications/ChatGPT.app/Contents/Resources/codex` completed the live read-only
  smoke test with exit 0. Its session header confirmed `gpt-6-astra` and `medium`.
- The fresh session read `AGENTS.md`, `CLAUDE.md`, and `docs/AGENT_WORKFLOW.md`,
  discovered all seven repository skills in its available-skills catalog, and
  correctly distinguished documentation, ECS, and rendering/WASM verification.
  It did not execute skill bodies or modify files. Some supplementary verification
  document output was truncated; this was not a full documentation audit.
- Representative model quality/time comparisons remain unmeasured. No engine
  edits were made for synthetic comparison tasks. Use the app-bundled Codex for
  Astra or the updated terminal CLI described below.
- The full engine gate and GPU/browser checks were skipped: only instructions,
  skills, and model configuration changed.
- With explicit user approval, updated the existing npm installation using
  `npm install -g @openai/codex@latest`. `/opt/homebrew/bin/codex` now reports
  0.154.0. A terminal read-only smoke test completed with exit 0, confirmed
  `gpt-6-astra` / `medium`, and listed all seven repository skills.
- Corrected cleanup verification to preserve pre-existing uncommitted contents,
  added Codex entrypoints to `FORKING.md`, and corrected its game count to five.
  Skill validation, `cargo check --locked`, and whitespace checks pass.

## Official references

- [Astra migration](https://developers.openai.com/api/docs/guides/latest-model)
- [Codex configuration](https://developers.openai.com/codex/config-reference)
- [Project instructions](https://developers.openai.com/codex/guides/agents-md)
- [Repository skills](https://developers.openai.com/codex/skills)
