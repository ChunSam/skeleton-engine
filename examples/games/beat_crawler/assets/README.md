# beat_crawler assets

## `soundtrack.wav` — the turn clock

One seamless 2.56 s bar (16 steps × 0.16 s), 22 050 Hz mono 16-bit PCM, looped by
`Audio::play_music`. The game does not read the pattern to decide turns; it *hears* the
kicks through `Audio::bands()`. `PATTERN` in `beat_crawler.rs` describes the same bar, and
exists only as the schedule `BEAT_WATCHDOG` falls back to when nothing can be heard.

**Provenance / license:** generated from scratch by `soundtrack.py` — sine arithmetic and a
seeded PRNG, no recording and no sample pack. It is therefore public domain (CC0) and safe
to commit to this MIT-licensed repository. Same reasoning as `src/audio/fixtures/README.md`.

**PCM, not Ogg/MP3, on purpose:** the loop has to be sample-exact. A lossy encoder pads the
stream, and `repeat_infinite` would replay that padding on every pass, drifting the bar
against the written schedule.

### Regenerating

```sh
python3 soundtrack.py soundtrack.wav          # the full mix
python3 soundtrack.py /tmp/kick.wav --only kick   # one layer, for measuring its bands
```

`--only kick|bass|hat|lead` renders a single layer. That is how the band layout was
established rather than assumed: the kick owns bands 0–1 (which share FFT bins at this
resolution and move together), and the bass — even after being moved up an octave — still
saturates bands 2–6 for most of the bar. `LOW_BANDS = 2` follows from that measurement.

## `hit.wav` — the melee impact

A 0.13 s one-shot, same format and same provenance rules: generated from scratch by `hit.py`
(a mid sine sweep plus first-differenced noise), so it is CC0 too. Fired with
`Audio::play_sfx_metered`, which is what lets a flurry overlap *and* still be measured.

**It is deliberately empty where the turn clock listens.** Measured over the whole clip with a
flat window:

| | 20–200 Hz | 200–800 Hz |
|---|---|---|
| `hit.wav` | **0.75%** | 97.87% |
| `soundtrack.wav` | 99.44% | 0.55% |

This is the right default for a game whose clock is a low-band detector, but be exact about what
it buys: **on native the two meters cannot leak into each other anyway**, because each is a tap on
its own channel and `bands()` never sees the mixer output. Firing `soundtrack.wav` itself as the
impact was tried and moved the kick count not at all. The separation matters for the **wasm**
backend, where several sources connect into one `AnalyserNode` and the browser mixes them — which
is the same property that makes the meter sum — and it costs nothing here.

**The 0.80 render level is load-bearing.** One voice reads the clip's own normalization and three
overlapping voices saturate the meter's 1.0 ceiling; rendering at 1.0 would put a single swing on
the ceiling too and leave the summing check (exit `7`) nothing to discriminate.

```sh
python3 hit.py hit.wav
```

### Why the bass sits on C3, not C2

The first mix put it an octave lower, right on top of the kick. Measured, every low band
sat pinned at full scale, the kick's transient disappeared into it, and **no threshold
separated them at all**. Moving the bass up is what a real mix does for the same reason. It
still overlaps the detector's window through its harmonics, so the two are separable
without being separated — which is the point of testing against a mix rather than two tones
chosen to be far apart.
