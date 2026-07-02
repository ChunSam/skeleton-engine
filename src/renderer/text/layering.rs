//! Z-interleaving of UI surfaces (rects/images) and layered text.
//!
//! The UI-primitive pass and the glyphon text pass are separate render passes, so z-ordering
//! *between* them is achieved by splitting the frame into alternating runs: draw the surfaces up
//! to a text's z, then that text batch, then the next surfaces, and so on. [`interleave_runs`]
//! computes those run lengths from the two **sorted** z lists; the caller draws sub-ranges of the
//! (single, pre-uploaded) surface instance buffer and one pooled glyphon batch per text group.

/// One alternating run: draw the next `surfaces` UI primitives, then the next `texts` layered
/// texts. Either count may be zero (e.g. text below every surface, or trailing surfaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Run {
    pub(crate) surfaces: usize,
    pub(crate) texts: usize,
}

/// Computes the alternating surface/text runs for one frame.
///
/// Inputs are the z values of the UI primitives and of the layered texts, **each sorted
/// ascending** (the caller sorts; primitives are already sorted by the instance-buffer build).
/// Ordering rule: a text at `z` draws **after** every surface with `surface_z <= z` and before
/// the first surface with `surface_z > z` — so on a tie the text wins (a widget label at the
/// widget's own z reads over the widget's background), while any surface strictly above hides it.
///
/// The run counts consume both lists exactly once: `sum(surfaces) == surface_zs.len()` and
/// `sum(texts) == text_zs.len()`.
pub(crate) fn interleave_runs(surface_zs: &[f32], text_zs: &[f32]) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut s = 0; // next surface not yet drawn
    let mut t = 0; // next text not yet drawn
    while t < text_zs.len() {
        let tz = text_zs[t];
        // Surfaces that must precede this text: z <= tz.
        let s2 = surface_zs[s..].partition_point(|&z| z <= tz) + s;
        // Texts in this batch: everything below the next (strictly higher) surface.
        let t2 = match surface_zs.get(s2) {
            Some(&next_sz) => text_zs[t..].partition_point(|&z| z < next_sz) + t,
            None => text_zs.len(),
        };
        runs.push(Run {
            surfaces: s2 - s,
            texts: t2 - t,
        });
        s = s2;
        t = t2;
    }
    if s < surface_zs.len() {
        runs.push(Run {
            surfaces: surface_zs.len() - s,
            texts: 0,
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals(runs: &[Run]) -> (usize, usize) {
        (
            runs.iter().map(|r| r.surfaces).sum(),
            runs.iter().map(|r| r.texts).sum(),
        )
    }

    #[test]
    fn no_texts_is_one_surface_run() {
        assert_eq!(
            interleave_runs(&[0.1, 0.5, 0.9], &[]),
            vec![Run {
                surfaces: 3,
                texts: 0
            }]
        );
        assert!(interleave_runs(&[], &[]).is_empty());
    }

    #[test]
    fn no_surfaces_is_one_text_run() {
        assert_eq!(
            interleave_runs(&[], &[0.2, 0.8]),
            vec![Run {
                surfaces: 0,
                texts: 2
            }]
        );
    }

    #[test]
    fn overlay_surface_draws_after_lower_text() {
        // Widget bg (0.5), its label (0.5 → after the bg, tie goes to text), then an overlay
        // rect (90.0) that must draw OVER the label, then the overlay's own text (90.001).
        let runs = interleave_runs(&[0.5, 90.0], &[0.5, 90.001]);
        assert_eq!(
            runs,
            vec![
                Run {
                    surfaces: 1,
                    texts: 1
                },
                Run {
                    surfaces: 1,
                    texts: 1
                },
            ]
        );
        assert_eq!(totals(&runs), (2, 2));
    }

    #[test]
    fn tie_puts_text_over_its_own_surface() {
        // Surface and text at the same z: surface first, then the text (label over its bg).
        assert_eq!(
            interleave_runs(&[1.0], &[1.0]),
            vec![Run {
                surfaces: 1,
                texts: 1
            }]
        );
    }

    #[test]
    fn text_below_every_surface_leads() {
        // A text at z 0.1 under two surfaces: text batch first, surfaces after (they cover it).
        assert_eq!(
            interleave_runs(&[0.5, 0.9], &[0.1]),
            vec![
                Run {
                    surfaces: 0,
                    texts: 1
                },
                Run {
                    surfaces: 2,
                    texts: 0
                },
            ]
        );
    }

    #[test]
    fn consecutive_texts_between_the_same_surfaces_share_a_batch() {
        // Three labels between the same two surface layers collapse into ONE text batch
        // (one glyphon prepare/render), not three.
        let runs = interleave_runs(&[0.0, 10.0], &[1.0, 2.0, 3.0]);
        assert_eq!(
            runs,
            vec![
                Run {
                    surfaces: 1,
                    texts: 3
                },
                Run {
                    surfaces: 1,
                    texts: 0
                },
            ]
        );
    }

    #[test]
    fn counts_always_consume_both_lists() {
        let surface_zs = [0.0, 0.0, 0.5, 0.9, 90.0, 90.0, 100.0];
        let text_zs = [0.0, 0.5, 0.5, 1.0, 90.0, 99.0, 100.0, 101.0];
        let runs = interleave_runs(&surface_zs, &text_zs);
        assert_eq!(totals(&runs), (surface_zs.len(), text_zs.len()));
        // Runs alternate correctly: no empty (0,0) runs.
        assert!(runs.iter().all(|r| r.surfaces > 0 || r.texts > 0));
    }
}
