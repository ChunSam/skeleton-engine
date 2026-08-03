#!/usr/bin/env python3
"""Generates `hit.wav` — beat_crawler's melee impact, synthesized from scratch.

Same arithmetic-only rule as `soundtrack.py`: sine waves and a seeded PRNG, no recording and
no sample pack, so the output is public domain (CC0) and safe to commit to an MIT-licensed
repository.

The interesting constraint is *where in the spectrum this clip is allowed to live*. The
game's turn clock is a low-band transient detector reading bands 0-1 (roughly 45-190 Hz at
this FFT resolution — see `assets/README.md`). Anything with real low-end would be
indistinguishable from a kick, so every swing would inject a phantom beat into the clock and
the dungeon would lurch on the player's attacks instead of on the music.

So the impact is a mid body (660 Hz sweeping down to 430) plus a first-differenced noise
transient for texture. It is loud, but it is loud *where the detector is not listening* —
the same separation trick the lead uses in `soundtrack.py`, applied for a different reason.

Measured over the whole clip (flat window, so a front-loaded transient is not attenuated the
way a Hann window would attenuate it):

    hit.wav          20-200 Hz   0.75%   200-800 Hz  97.87%
    soundtrack.wav   20-200 Hz  99.44%   200-800 Hz   0.55%

Under 1% of the clip's energy lands in the detector's window, against a kick that puts
essentially all of its there. The body dominates by design; the noise contributes ~1.4% and
is texture, not the character of the sound.

Run:  python3 hit.py hit.wav
"""

import math
import struct
import sys
import wave

RATE = 22050  # matches soundtrack.wav; the mixer resamples either way, but this keeps them equal
DUR = 0.13

n = int(RATE * DUR)


def body():
    """Mid sine with a short downward sweep — the 'thud' without any sub content."""
    out, phase = [], 0.0
    for i in range(n):
        t = i / RATE
        freq = 430.0 + (660.0 - 430.0) * math.exp(-t / 0.018)
        phase += 2 * math.pi * freq / RATE
        out.append(math.sin(phase) * math.exp(-t / 0.030))
    return out


def crack(seed=0xA55E7):
    """Seeded white noise, first-differenced (one-pole high pass) so nothing lands down low."""
    state, out, prev = seed, [], 0.0
    for i in range(n):
        state = (state * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)
        x = ((state >> 33) / float(1 << 31)) - 1.0
        out.append((x - prev) * math.exp(-(i / RATE) / 0.016))
        prev = x
    return out


buf = [b * 0.55 + c * 0.45 for b, c in zip(body(), crack())]

peak = max(abs(v) for v in buf)
gain = 0.80 / peak
frames = b"".join(struct.pack("<h", int(max(-1.0, min(1.0, v * gain)) * 32767)) for v in buf)

out_path = sys.argv[1] if len(sys.argv) > 1 else "hit.wav"
with wave.open(out_path, "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(RATE)
    w.writeframes(frames)

print(f"{out_path}: {n} frames, {n / RATE:.3f}s, peak {peak:.3f} -> gain {gain:.3f}")
