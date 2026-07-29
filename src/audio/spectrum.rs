//! Native spectrum analysis: a small radix-2 FFT and the log-spaced band folding on top of it.
//!
//! The wasm backend gets a spectrum for free — a Web Audio `AnalyserNode` does the transform in
//! the browser. Native has no equivalent: rodio does not analyze, and no dependency in the tree
//! provides an FFT. Rather than pull in a DSP crate for one 1024-point transform, this is a plain
//! iterative Cooley–Tukey implementation, in the same spirit as the hand-written SplitMix64 in
//! [`Rng`](crate::Rng) and the shadowcasting in [`FovMap`](crate::FovMap).
//!
//! # Why this is safe to hand-write
//!
//! An FFT is one of the very few pieces of DSP that can be checked *exactly* rather than by
//! eyeball: feed it a sine at a known frequency and the energy must land in the matching bin.
//! The tests below do that, plus Parseval's theorem (energy is conserved) and a DC case — so the
//! transform is pinned by mathematics rather than by a golden file.

use std::f32::consts::PI;

/// Applies a Hann window in place.
///
/// Without a window, a tone whose period does not divide the buffer length exactly has its energy
/// smeared across every bin (spectral leakage), and a spectrum display looks like noise rather
/// than a peak. Hann is the standard general-purpose choice.
pub(crate) fn apply_hann(samples: &mut [f32]) {
    let n = samples.len();
    if n < 2 {
        return;
    }
    let denom = (n - 1) as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / denom).cos());
        *s *= w;
    }
}

/// In-place iterative radix-2 Cooley–Tukey FFT.
///
/// `re` and `im` must be the same length and a power of two; anything else is left untouched
/// (a silently-wrong transform would be far worse than no transform).
pub(crate) fn fft_in_place(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    if n != im.len() || n < 2 || !n.is_power_of_two() {
        return;
    }

    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Butterfly stages.
    let mut len = 2usize;
    while len <= n {
        let angle = -2.0 * PI / len as f32;
        let (wr, wi) = (angle.cos(), angle.sin());
        let half = len / 2;
        for start in (0..n).step_by(len) {
            let (mut cur_r, mut cur_i) = (1.0f32, 0.0f32);
            for k in 0..half {
                let (ar, ai) = (re[start + k], im[start + k]);
                let (br, bi) = (re[start + k + half], im[start + k + half]);
                let (tr, ti) = (br * cur_r - bi * cur_i, br * cur_i + bi * cur_r);
                re[start + k] = ar + tr;
                im[start + k] = ai + ti;
                re[start + k + half] = ar - tr;
                im[start + k + half] = ai - ti;
                let next_r = cur_r * wr - cur_i * wi;
                cur_i = cur_r * wi + cur_i * wr;
                cur_r = next_r;
            }
        }
        len <<= 1;
    }
}

/// Transforms `samples` (windowed and consumed) into `bands` normalized log-spaced magnitudes.
///
/// Bands are spaced **logarithmically** because that is how pitch is perceived: linear bands give
/// a display where the bottom two octaves share one bar and the top octave gets half the screen.
/// Each band takes the **loudest** FFT bin inside it — not the mean, which would average a narrow
/// tone sitting in a wide high-frequency band into invisibility — then goes through the shared
/// [`normalized_db`](crate::audio_analysis::normalized_db) so the values mean the same thing they
/// do on the web backend.
///
/// `scratch_re`/`scratch_im` are caller-owned so this allocates nothing — it runs on the audio
/// thread.
pub(crate) fn spectrum_into(
    samples: &[f32],
    scratch_re: &mut [f32],
    scratch_im: &mut [f32],
    bands: &mut [f32],
) {
    let n = samples.len();
    if n < 2 || !n.is_power_of_two() || scratch_re.len() != n || scratch_im.len() != n {
        bands.fill(0.0);
        return;
    }
    scratch_re[..n].copy_from_slice(samples);
    apply_hann(scratch_re);
    scratch_im.fill(0.0);
    fft_in_place(scratch_re, scratch_im);

    // Only the first half of the spectrum is meaningful for a real input (the rest mirrors it).
    // Bin 0 is DC and carries no pitch, so bands start at bin 1.
    let usable = n / 2;
    if usable < 2 || bands.is_empty() {
        bands.fill(0.0);
        return;
    }
    // Normalize by the window's coherent gain (Hann sums to n/2) so a full-scale sine reaches
    // roughly unit magnitude rather than scaling with the transform length.
    let norm = 2.0 / (n as f32 * 0.5);

    let n_bands = bands.len();
    for (b, slot) in bands.iter_mut().enumerate() {
        // Band edges come from the shared definition, so the wasm backend's bands — folded from a
        // browser's FFT — cover exactly the same frequencies.
        let (start, end) = crate::audio_analysis::log_band_range(b, n_bands, usable);
        let mut peak = 0.0f32;
        for k in start..end {
            let mag = (scratch_re[k] * scratch_re[k] + scratch_im[k] * scratch_im[k]).sqrt() * norm;
            if mag > peak {
                peak = mag;
            }
        }
        // Peak, not mean, within a band: a narrow tone inside a wide high-frequency band would be
        // averaged into invisibility otherwise.
        *slot = crate::audio_analysis::normalized_db(peak);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    const N: usize = 1024;

    /// A pure cosine at exactly `bin` cycles per buffer — so its energy lands in that FFT bin.
    fn cosine_at_bin(bin: usize, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (TAU * bin as f32 * i as f32 / n as f32).cos())
            .collect()
    }

    fn magnitudes(samples: &[f32]) -> Vec<f32> {
        let mut re = samples.to_vec();
        let mut im = vec![0.0; samples.len()];
        fft_in_place(&mut re, &mut im);
        re.iter()
            .zip(&im)
            .map(|(r, i)| (r * r + i * i).sqrt())
            .collect()
    }

    #[test]
    fn a_sine_puts_its_energy_in_the_matching_bin() {
        // The exact check an FFT allows and eyeballing does not: energy at bin 64, nowhere else.
        let mags = magnitudes(&cosine_at_bin(64, N));
        let peak_bin = mags[..N / 2]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(peak_bin, 64, "expected the peak at bin 64, got {peak_bin}");
    }

    #[test]
    fn a_different_frequency_moves_the_peak_correspondingly() {
        for bin in [8usize, 32, 100, 255] {
            let mags = magnitudes(&cosine_at_bin(bin, N));
            let peak = mags[..N / 2]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap();
            assert_eq!(peak, bin, "bin {bin} landed at {peak}");
        }
    }

    #[test]
    fn dc_lands_in_bin_zero_only() {
        let mags = magnitudes(&vec![1.0f32; N]);
        assert!((mags[0] - N as f32).abs() < 1e-2, "DC bin {}", mags[0]);
        for (k, m) in mags.iter().enumerate().take(N / 2).skip(1) {
            assert!(*m < 1e-2, "bin {k} should be empty, got {m}");
        }
    }

    #[test]
    fn energy_is_conserved_parsevals_theorem() {
        // sum(|x|^2) == (1/N) * sum(|X|^2). A transform that scaled or dropped terms fails here
        // even when the peak bin happens to be right.
        let x = cosine_at_bin(37, N);
        let time_energy: f32 = x.iter().map(|v| v * v).sum();
        let freq_energy: f32 = magnitudes(&x).iter().map(|m| m * m).sum::<f32>() / N as f32;
        let rel = (time_energy - freq_energy).abs() / time_energy;
        assert!(
            rel < 1e-3,
            "energy drift {rel} ({time_energy} vs {freq_energy})"
        );
    }

    #[test]
    fn a_non_power_of_two_is_left_untouched_rather_than_silently_wrong() {
        let mut re = vec![1.0f32, 2.0, 3.0];
        let mut im = vec![0.0f32; 3];
        let before = re.clone();
        fft_in_place(&mut re, &mut im);
        assert_eq!(
            re, before,
            "a bad length must not produce a bogus transform"
        );
    }

    #[test]
    fn the_hann_window_tapers_to_zero_at_both_ends() {
        let mut v = vec![1.0f32; 8];
        apply_hann(&mut v);
        assert!(v[0].abs() < 1e-6, "start {}", v[0]);
        assert!(v[7].abs() < 1e-6, "end {}", v[7]);
        assert!(v[4] > 0.8, "middle should be near unity, got {}", v[4]);
    }

    #[test]
    fn a_low_tone_lights_a_low_band_and_a_high_tone_a_high_one() {
        // The end-to-end behaviour a visualizer depends on: pitch maps to bar position.
        let mut re = vec![0.0; N];
        let mut im = vec![0.0; N];
        let mut low = vec![0.0; 16];
        let mut high = vec![0.0; 16];
        spectrum_into(&cosine_at_bin(4, N), &mut re, &mut im, &mut low);
        spectrum_into(&cosine_at_bin(200, N), &mut re, &mut im, &mut high);

        let arg_max = |v: &[f32]| {
            v.iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap()
        };
        let (lo_band, hi_band) = (arg_max(&low), arg_max(&high));
        assert!(
            lo_band < hi_band,
            "a 4-cycle tone should peak below a 200-cycle tone, got {lo_band} vs {hi_band}"
        );
    }

    #[test]
    fn silence_produces_no_bands() {
        let mut re = vec![0.0; N];
        let mut im = vec![0.0; N];
        let mut bands = vec![1.0; 16]; // pre-filled, so a no-op would be visible
        spectrum_into(&vec![0.0; N], &mut re, &mut im, &mut bands);
        assert!(bands.iter().all(|b| *b == 0.0), "{bands:?}");
    }
}
