# Audio codec test fixtures

Tiny synthesized audio files used by the decode tests in `src/audio/playback.rs`
(`codec_decode_tests`) to prove the engine's enabled `rodio` codec features
(`wav` / `vorbis` / `mp3`) actually decode.

**Provenance / license:** each file is the *same* ~0.15 s, 22 050 Hz, mono, 440 Hz
sine wave, generated from scratch — no third-party audio. They are therefore in the
public domain (CC0) and safe to commit to this MIT-licensed repository. This is why
the decode tests use committed fixtures rather than any licensed sample.

| File | Codec | Encoder |
|------|-------|---------|
| `tone.wav` | PCM (RIFF/WAVE) | Python `wave` (stdlib) |
| `tone.ogg` | Vorbis in Ogg | `oggenc -q 0` (vorbis-tools) |
| `tone.mp3` | MPEG-1/2 Layer III | `lame -m m -b 32` |

## Regenerating

```sh
python3 - tone.wav <<'PY'
import sys, wave, math, struct
rate, dur, freq = 22050, 0.15, 440.0
n = int(rate * dur)
with wave.open(sys.argv[1], "wb") as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(rate)
    w.writeframes(b"".join(
        struct.pack("<h", int(0.35 * 32767 * math.sin(2*math.pi*freq*i/rate)))
        for i in range(n)))
PY
oggenc -Q -q 0 tone.wav -o tone.ogg
lame --quiet -m m -b 32 tone.wav tone.mp3
```

`oggenc` ships with `vorbis-tools`; `lame` is its own package (both via Homebrew on
macOS: `brew install vorbis-tools lame`). CI never needs an encoder — it only
*decodes* these committed files.
