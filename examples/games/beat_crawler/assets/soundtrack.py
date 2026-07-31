#!/usr/bin/env python3
"""Generates `soundtrack.wav` — beat_crawler's one-bar loop, synthesized from scratch.

Everything here is arithmetic on sine waves and a seeded PRNG: no third-party audio, no
sample pack, no recording. The output is therefore public domain (CC0) and safe to commit
to an MIT-licensed repository. Same reasoning as `src/audio/fixtures/README.md`.

The point of this track is that it is a *mix*, not two well-separated tones. The bass
sustains in the same 45-190 Hz window the kick detector reads, so a kick has to be found
as a transient riding a floor rather than as energy-vs-silence. That is the whole reason
the example ships a track instead of scheduling `play_tone_on_channel` calls.

Run:  python3 soundtrack.py soundtrack.wav
"""

import math
import struct
import sys
import wave

RATE = 22050
STEPS = 16
STEP_SECS = 0.16  # must match STEP_SECS in beat_crawler.rs
STEP_LEN = round(RATE * STEP_SECS)  # 3528 samples — exact, so the loop is seamless
TOTAL = STEPS * STEP_LEN

# Which steps carry which layer. These mirror `PATTERN` in beat_crawler.rs — the track and
# the schedule describe the same bar, which is what lets the watchdog fall back to the
# schedule when nothing can be heard. Gameplay still learns the beat by listening.
KICK_STEPS = (0, 4, 8, 12)
LEAD_STEPS = {2: 587.33, 6: 440.00, 7: 659.25, 10: 587.33, 13: 880.00, 15: 659.25}
HAT_STEPS = tuple(range(1, STEPS, 2))
# One bass note per 4 steps, an octave above the kick's fundamental. This is a mixing
# decision, not a way of making the detector's job easy: a bass sitting *on* the kick
# (C2/F2/G2 at this level) pins bands 0-6 at full scale, and a saturated band cannot show a
# transient — measured, the kick became invisible and every threshold fired on bass wobble.
# Real mixes leave the sub to the kick for the same reason. The bass still overlaps the
# detector's window through its harmonics, so the two are separable but not separated.
BASS_NOTES = (130.81, 130.81, 174.61, 196.00)  # C3, C3, F3, G3
BASS_GAIN = 0.10

buf = [0.0] * TOTAL


def add(start, samples):
    for i, s in enumerate(samples):
        j = (start + i) % TOTAL  # wrap, so a tail that runs past the bar lands on the loop
        buf[j] += s


def kick(dur=0.11):
    """Pitch-swept sine, 150 Hz -> 48 Hz, with a fast exponential body decay."""
    n = int(RATE * dur)
    out, phase = [], 0.0
    for i in range(n):
        t = i / RATE
        freq = 48.0 + (150.0 - 48.0) * math.exp(-t / 0.022)
        phase += 2 * math.pi * freq / RATE
        out.append(math.sin(phase) * math.exp(-t / 0.055))
    return out


def bass(freq, dur):
    """Sine plus a little second harmonic, with a tremolo that never reaches zero."""
    n = int(RATE * dur)
    out = []
    for i in range(n):
        t = i / RATE
        w = math.sin(2 * math.pi * freq * t) + 0.30 * math.sin(4 * math.pi * freq * t)
        env = 0.72 + 0.28 * math.sin(2 * math.pi * t / (STEP_SECS * 2))
        out.append(w * env)
    return out


def lead(freq, dur=0.10):
    """Odd-harmonic (square-ish) blip — clearly audible, well above the low window."""
    n = int(RATE * dur)
    out = []
    for i in range(n):
        t = i / RATE
        w = (
            math.sin(2 * math.pi * freq * t)
            + math.sin(6 * math.pi * freq * t) / 3.0
            + math.sin(10 * math.pi * freq * t) / 5.0
        )
        out.append(w * math.exp(-t / 0.035))
    return out


def hat(seed, dur=0.045):
    """Seeded white noise, first-differenced (a one-pole high pass) so it stays up top."""
    n = int(RATE * dur)
    state, out, prev = seed, [], 0.0
    for i in range(n):
        state = (state * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)
        x = ((state >> 33) / float(1 << 31)) - 1.0
        out.append((x - prev) * math.exp(-(i / RATE) / 0.012))
        prev = x
    return out


# ── Lay the bar down ────────────────────────────────────────────────────────────────────
# `--only <layer>` renders one layer in isolation. Used to measure which spectrum bands each
# layer actually occupies, rather than assuming it from the note names.
only = None
if "--only" in sys.argv:
    only = sys.argv[sys.argv.index("--only") + 1]


def want(layer):
    return only is None or only == layer


if want("kick"):
    for s in KICK_STEPS:
        add(s * STEP_LEN, [v * 1.00 for v in kick()])
if want("lead"):
    for s, f in LEAD_STEPS.items():
        add(s * STEP_LEN, [v * 0.17 for v in lead(f)])
if want("hat"):
    for i, s in enumerate(HAT_STEPS):
        add(s * STEP_LEN, [v * 0.14 for v in hat(0x5EED + i)])
if want("bass"):
    for i, f in enumerate(BASS_NOTES):
        add(i * 4 * STEP_LEN, [v * BASS_GAIN for v in bass(f, STEP_SECS * 4)])

peak = max(abs(v) for v in buf)
gain = 0.85 / peak
frames = b"".join(struct.pack("<h", int(max(-1.0, min(1.0, v * gain)) * 32767)) for v in buf)

out_path = sys.argv[1] if len(sys.argv) > 1 else "soundtrack.wav"
with wave.open(out_path, "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(RATE)
    w.writeframes(frames)

print(f"{out_path}: {TOTAL} frames, {TOTAL / RATE:.3f}s, peak {peak:.3f} -> gain {gain:.3f}")
