# Editor allocation measurements — v0.159.6

The final two observations from the 2026-09-02 editor review were measured on
2026-09-14/15. The large costs were the data-table grid, repeated entity-sort keys and
displaying a large component clipboard. Those paths are optimized; the smaller observations
below are measured and retained. These are editor paths, not the game update loop.

## Reproduce

```sh
python3 scripts/editor_alloc_probe.py --check
```

The script copies engine sources into a temporary directory, injects
`scripts/probes/editor_alloc.rs` into the private editor UI module, and exposes the private
sort only in that copy. It runs exactly one release-mode library test with a thread-local
counting allocator. The temporary sources are removed afterward; Cargo artifacts are shared
with this checkout. No public measurement API or production allocator is added.

Measurements use macOS arm64, Rust 1.95.0, pinned egui 0.34.3, a 900×500 logical-pixel viewport,
three warmup calls and nine measured calls. The table reports the median-duration call's
allocation count, requested bytes and microseconds. An allocation/reallocation counts as one
call; bytes are the **full requested size**, including reallocations, not allocator buckets,
peak live memory or an OS memory measurement. Setup, image decoding and component copying
occur outside the display measurements. Real paste actions are measured separately.

Panel timings include egui CPU layout/shapes and disposal of its output. Sort/filter/bounds/
held-gizmo cases call the real private helpers. They do not include tessellation, GPU rendering,
window presentation or the full editor frame. Times from one machine are indicative, not an
FPS guarantee or a timing assertion. The allocation guards have headroom for small egui changes.

## Before and after

The baseline is `c95d03ef836e6ad991b2c628cf2c86c1a07ec61e` (before v0.159.6).
The same fixture sizes and measured calls were used before and after. Additional positive
controls execute outside those measurements.

Values are **allocation calls / requested bytes / µs** per measured call.

| Workload | Before | After |
|---|---:|---:|
| `table/0` | 97 / 16,043 / 27 | 101 / 17,925 / 34 |
| `table/10` | 340 / 53,299 / 98 | 393 / 61,071 / 128 |
| `table/100` | 2,426 / 323,319 / 684 | 715 / 108,227 / 228 |
| `table/1000` | 23,129 / 2,936,887 / 4,764 | 715 / 108,227 / 133 |
| `table/10000` | 230,133 / 29,141,767 / 59,132 | 715 / 108,227 / 201 |
| `sort/Index/1000` | 1 / 8,000 / 0 | 1 / 8,000 / 0 |
| `sort/Name/10` | 181 / 2,240 / 7 | 12 / 520 / 0 |
| `sort/Name/1000` | 42,114 / 521,344 / 1,917 | 1,002 / 52,000 / 88 |
| `sort/Kind/10` | 181 / 2,240 / 39 | 12 / 600 / 4 |
| `sort/Kind/1000` | 42,114 / 521,344 / 9,564 | 1,002 / 60,000 / 460 |
| `clipboard/0` | 106 / 14,436 / 12 | 104 / 14,412 / 17 |
| `clipboard/10000` | 106 / 744,435 / 24 | 104 / 14,412 / 14 |

The table has boolean, integer and string columns. It now borrows cells and submits only
visible rows; zero-row and ten-row cases are controls, not expected speedups. The extra stable
cell scopes add a small cost at ten rows. Column headers and complex-value labels stay on one
line, and string-field margins plus rounded row heights keep the virtual scroll geometry
consistent. Cell IDs are independent of the visible range, so a visible editing cell keeps focus
when scrolling. Add/delete/save/reload actions retain the collect-then-apply behavior.

Sorting caches keys once per entity, including the component-kind classification. It preserves
case-insensitive Unicode ordering, stable ties and the generated label for an untagged entity.
Index mode still copies the incoming list without sorting.

The clipboard fixture calls the real serde copy operation. Its `ron::Value` contains a large
serialized string: the original clone count stayed constant, while requested bytes grew with
the payload. Display now borrows the name. A real paste still deserializes and allocates; this
is expected work on an action, not a frame cost eliminated by the display optimization.

## Measured costs retained

Same units: **allocation calls / requested bytes / µs**.

| Workload | Measured result |
|---|---:|
| `assets/100` | 1,444 / 131,446 / 145 |
| `assets/1000` | 14,050 / 1,175,094 / 1,437 |
| `component-list/10000` | 110 / 15,068 / 14 |
| `filter/1000` | 2,002 / 18,018 / 73 |
| `bounds/1000` | 1 / 16,000 / 42 |
| `bounds-colliders/1000` | 2 / 36,000 / 83 |
| `gizmo-hold/1000` | 9 / 16,352 / 156 |
| `grid/default` | 25 / 15,502 / 3 |
| `grid/cursor` | 30 / 15,878 / 4 |
| `sort-untagged/1000` | 2,002 / 75,890 / 496 |
| `paste/10000` | 10,019 / 2,946,677 / 2,343 |

Filter numbers include one positive match before the batch: the 1,000-name case therefore
contains 1,001 filter calls. The held-gizmo case moves every selected entity on all twelve
warmup/measured calls; vector growth explains why one temporary vector can require several
allocator calls. Bounds fixtures verify the emitted shape count, both with and without colliders.
The cursor-grid case includes a real pointer event and its readout, as well as egui overhead.

The populated Inspector case lists one registered component containing 10,000 payload entries,
including the copyable-name set and the normal registered-factory dropdown. It confirms presence
checks do not serialize that payload. It is not a benchmark of every possible component registry.

The asset browser remains linear: `image_list()` copies metadata and the panel builds every
entry. Revisit it if a representative project keeps hundreds/thousands of images loaded and a
full editor-frame profile identifies the open Assets tab as a bottleneck. Variable-height labels
need a deliberate layout decision before virtualizing that panel. For filtering, bounds, grid,
group dragging and Inspector bookkeeping, revisit when a representative selection/registry or
frame profile exceeds the measured workload or identifies a material budget problem. No claim
of zero allocations is made, and these observations are not a new automatic work queue.

## Regression evidence

`--check` bounds the 10,000-row table's requested bytes and its growth relative to 100 rows,
clipboard display growth relative to an empty payload, and named-sort allocations/bytes per
entity. It is an opt-in performance check, separate from `scripts/verify.sh`.

The probe checks actual table/header text, clipboard and component-copy buttons, loaded image
labels, sorted-list lengths, successful paste contents, emitted bounds and moved group members.
Empty/small fixtures, untagged-sort fallback and actual paste work prevent a cheap early return
from being mistaken for a successful optimization.

Each old implementation was restored independently in the disposable source copy, leaving the
allocation test intact. All three exited **101** at their corresponding allocation guard:

```sh
python3 scripts/editor_alloc_probe.py --check --restore table --baseline-ref c95d03e
python3 scripts/editor_alloc_probe.py --check --restore sort --baseline-ref c95d03e
python3 scripts/editor_alloc_probe.py --check --restore clipboard --baseline-ref c95d03e
```

The headless data-table tests exercise real pointer/text/scroll events, editing after scrolling,
deletion of the intended row, adding a row, an empty table, a larger font, the last row and scroll
distance against the actual row spacing. Independent cell-ID, wrong-row and row-height sabotages
exit **101** in the corresponding assertions; source restoration is checked byte-for-byte.
The last-row check alone did not detect an underestimated row height; the scroll-distance test
does, so the evidence is attributed to that assertion. Sort tests also cover Unicode ties and
the untagged fallback.

These input tests and the existing engine verification gate do not prove windowed visual
behavior. No browser or real-window performance measurement is claimed.

Validation on 2026-09-15: `SKELETON_MUTE=1 ./scripts/verify.sh` exited **0**, including native
fmt/clippy/tests, WASM builds/clippy/examples, doctests, rustdoc and all five example selftests.
The final allocation probe with `--check` also exited **0**. Probe formatting, Python syntax,
version consistency, new documentation links and `git diff --check` passed.
