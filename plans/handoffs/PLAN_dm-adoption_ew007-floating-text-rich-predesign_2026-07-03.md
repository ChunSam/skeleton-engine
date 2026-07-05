# EW-007 pre-design — FloatingText bold / rich-markup passthrough

**Date:** 2026-07-03
**Status:** DESIGN ONLY — nothing implemented, nothing filed on the board (filing is the
game's move per the work-around-first rule; pre-filing engine-side would invert the pipe).
Written per dm-adoption seq-4 Open Question #3, user-approved ("b", 2026-07-03), so a
future EW-007 can be served same-day like EW-006 was.
**Chain:** `dm-adoption` (design appendix to seq 4 — `HANDOFF_dm-adoption_ew-verified-coordination_2026-07-03.md`)

---

## The anticipated request

From the game's EW-004 `Verified` reply (board, game main): the migrated floats render
**regular weight**, while the old hand-rolled `popups.rs` drew `[b]` rich-markup **bold**.
"If it washes out on the colored tiles we may come asking for a weight/rich option on
`FloatingText`." The probe is in their batched usertest queue (18+ pending as of 2026-07-03;
their current session is doing PB-003 layout work, so the usertests have not run yet).

## Key finding: the engine already has rich markup — the gap is ONE passthrough

Seq 4 assessed this as "non-trivial (cosmic-text attrs plumbing through `DrawText`)".
**That assessment is wrong.** Code reading (2026-07-03, main `fa20667`):

- `DrawText.rich: bool` + `.rich()` builder already exist (`src/renderer/text/queue.rs:38,109`)
  — "Interprets `[color=#RRGGBB]...[/color]`, `[b]...[/b]`, and `[i]...[/i]` tags."
- The parser is `src/renderer/text/rich_text.rs::parse_rich_text`: `[b]` →
  `Attrs.weight(Weight::BOLD)`, `[i]` → `Style::Italic`, `[color=#RRGGBB[AA]]` → span color;
  nesting via depth counters; unit-tested in `src/renderer/text/tests.rs`.
- The renderer applies it via `buf.set_rich_text(...)` (`src/renderer/text/renderer.rs:450-462`),
  default attrs `Attrs::new().family(Family::SansSerif)`.
- `label_pass.rs` already sets `.rich()` — Labels render markup today; the
  `settings_menu` game example uses it. This is exactly how the game's OLD popups got bold:
  they pushed `DrawText::new(..).rich()` directly.

The only gap: `FloatingTextSystem` builds `DrawText::centered(..)` (`src/floating_text.rs:225`)
and never sets `rich`. So the serve is the **EW-004 `with_z` shape again**, one field smaller.

## Recommended API (Option A)

```rust
// src/floating_text.rs
pub struct FloatingText {
    // ...existing fields...
    /// Interpret `[b]`/`[i]`/`[color=#RRGGBB]` tags in `text` (DrawText rich markup).
    pub rich: bool,   // default false = byte-identical
}
pub fn with_rich(mut self) -> Self { self.rich = true; self }

// FloatingTextSystem::run, next to the existing z passthrough:
if ft.rich { draw = draw.rich(); }
```

- Non-breaking: `false` default; `FloatingText` has no serde (transient, like `HitFlash`),
  clone registration covers the new field automatically. MINOR bump.
- Game adoption: `floats.rs::spawn_float` wraps call-site text in `[b]…[/b]` (or grows a
  `bold: bool` param) + `.with_rich()` — their exact old visual shape restored.
- Estimate: **~30–60 min** including tests + example — same-day serve is trivially safe.

### Tests / acceptance (VISION loop)

- Unit: passthrough test mirroring the `with_z` test at `src/floating_text.rs:378-399`
  (queued `DrawText.rich == true`; default stays `false`).
- Example: `floating_text` — spawn the crit float bold (`[b]`), add a `B` toggle;
  headless auto-mode includes one bold float; eyeball the capture (bold visibly heavier
  next to a regular float on macOS, where system Helvetica/SF bold resolves).
- A CI render test asserting bold > regular pixel coverage is **environment-dependent**
  (needs a bold sans face on the runner; ubuntu usually has DejaVu Sans Bold, but don't
  bet the gate on it). If added, assert tolerant `>=` with the strict check local-only.
  The reliable acceptance = unit passthrough + capture eyeball.

### Caveats to document on `with_rich` (doc comment + CHANGELOG)

1. **Bold is face selection, not synthesis.** cosmic-text does no faux-bold. Native:
   `FontSystem::new()` loads system fonts → `Family::SansSerif` + `Weight::BOLD` resolves
   to a real bold face (why the game's old `[b]` worked on macOS). **Wasm: fontdb's
   `load_system_fonts` is a no-op** and every bundled face (DejaVuSans, NotoSansKR subset,
   NotoSansHebrew) is Regular-only → `[b]` silently renders regular on web unless the game
   loads a bold TTF via `FontData`/`ExtraFonts`. DM is native-only today — fine, but say it.
2. **`[color]` spans bypass the fade.** `FloatingText`'s fade multiplies the alpha of
   `DrawText.color`, which reaches glyphon as the *default* color; a `[color=…]` span pins
   its own RGBA and will NOT fade out. Guidance: use `[b]`/`[i]` inside floats; set color
   via `FloatingText.color` (per-float, fades correctly).
3. **Rich text skips the plain shaped-buffer cache** (`renderer.rs:382`) → re-shaped per
   frame. Floats are few and short-lived (~1 s); negligible, but worth one sentence.
4. **No tag escaping.** Literal `[b]` in text is consumed by the parser. Floats carry
   game-authored numbers — non-issue; don't build escaping for this.

## Rejected / deferred alternatives

- **Option B — whole-run `weight` field on `DrawText`/`FloatingText`:** duplicates what
  markup already does, and `PlainTextCacheKey` would need `weight` added (or cached buffers
  collide across weights). Only worth it if the game explicitly wants weight without tags.
- **Option C — extend the markup (sizes, outline/shadow):** out of scope. NOTE: if the
  usertest verdict is "hard to read on colored tiles", an **outline/shadow** option may be
  the better legibility tool than bold — wait for the actual finding before designing.

## Adjacent nit (fix in the same PR)

- `src/renderer/text.rs:1` module doc says "backed by glyphon 0.6"; Cargo.toml pins
  `glyphon = "0.11"`. One-word doc fix.

## Serve checklist when EW-007 lands

1. Board reply `Acknowledged` → branch `feat/floating-text-rich` → Option A impl + tests +
   example toggle (`/add-feature-example` is overkill for a passthrough; follow the EW-004
   #337 shape) → `/ship` MINOR → `/land-pr` Async → board `Shipped (vX.Y.Z)` + reply with
   the caveats above (esp. #1 wasm, #2 fade).
2. If the request turns out to be legibility-not-boldness, counter-propose outline/shadow
   on the board thread before building (Option C note).
