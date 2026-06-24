# Agent working notes — skeleton-engine

Working heuristics for an agent editing this codebase. Referenced from `CLAUDE.md`
("Agent working notes"); kept here to keep `CLAUDE.md` ≤200 lines.

## Context management

The longer a session runs, the more accumulated context degrades response quality. Split the approach by task type:

| Situation | Recommended approach |
|------|-----------|
| Single-file edit (clear requirements) | Edit directly in the main session |
| Feature spanning multiple files | Split out into a Task subagent |
| Exploration needs 3+ files | Explore subagent |
| Writing code after a long conversation | Task subagent (avoid context pollution) |

## Efficient exploration

- Locate symbols/keywords with `grep` before reading whole files
- If the path is already known, use Read directly (no Explore subagent needed)
- Reading order: `src/lib.rs` → module map → narrow down to the target file

## Subagent prompt principles

A subagent starts without knowing the current conversation context. Always include in the prompt:

1. **Paths to edit** (absolute paths)
2. **Patterns to apply** — pass a summary of `CLAUDE.md`'s core-pattern sections (borrow workaround, layer separation, etc.) and `docs/PATTERNS.md`
3. **Expected result** — what behavior should change
