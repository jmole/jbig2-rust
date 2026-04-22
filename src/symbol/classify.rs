//! Lossy classifier — coarsens an identity dictionary by weighted-XOR
//! distance.
//!
//! The identity classifier ([`crate::symbol::identity`]) folds together
//! components whose bitmaps are pixel-identical. That by itself already
//! captures the majority of redundancy in scanned text; the lossy pass
//! below goes one step further and merges near-identical components (e.g.
//! two slightly different "e" glyphs from the same font at the same point
//! size) onto a single representative bitmap. The resulting JBIG2 file is
//! **lossy** in the strict pixel sense — visual intent is preserved but a
//! handful of pixels per glyph may move.
//!
//! # Distance metric
//!
//! We use a normalised weighted XOR distance:
//!
//! ```text
//! d(a, b) = popcount(a XOR b_shifted) / min(area(a), area(b))
//! ```
//!
//! where `b_shifted` is `b` translated so that its centre of mass aligns
//! with `a`'s. For unpadded (and unshifted) inputs this simplifies to the
//! raw XOR popcount fraction. Two bitmaps with identical shapes score 0;
//! two completely different shapes of the same pixel count score ≥1.
//!
//! Matches are only considered between candidates whose width and height
//! agree to within 1 pixel each — this costs O(n·bucket_size) rather than
//! O(n²) and is what `jbig2enc` does as a first-pass filter.

use std::collections::HashMap;

use crate::bitmap::Bitmap;
use crate::symbol::cc::Component;
use crate::symbol::identity::IdentityClassification;

/// Output of running the lossy classifier on a component list.
#[derive(Clone, Debug)]
pub struct LossyClassification {
    /// Representative symbols in assignment order.
    pub symbols: Vec<Bitmap>,
    /// For each input component, the index into `symbols`.
    pub instance_symbol: Vec<u32>,
    /// Number of lossy (non-identity) merges performed.
    pub lossy_merge_count: u32,
}

/// Run the lossy classifier. `threshold` is the minimum fraction of
/// pixels that must agree for a merge, following the `jbig2enc` convention:
/// `1.0` = only identity matches, `0.85` = up to 15% of pixels may
/// disagree. Typical values are `0.85..0.97`.
pub fn classify_lossy(components: &[Component], threshold: f32) -> LossyClassification {
    // First pass: identity dedup (fast, exact).
    let identity = IdentityClassification::from_classify(
        crate::symbol::identity::classify_identity(components),
    );

    // Interpret threshold as "maximum allowed disagreement" internally.
    let max_disagreement = 1.0 - threshold.clamp(0.0, 1.0);

    if threshold >= 1.0 {
        return LossyClassification {
            symbols: identity.symbols,
            instance_symbol: identity.instance_symbol,
            lossy_merge_count: 0,
        };
    }

    // Second pass: merge near-identical identity symbols. We bucket symbols
    // by (width, height) so the search neighbourhood is bounded.
    let mut buckets: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (i, s) in identity.symbols.iter().enumerate() {
        buckets
            .entry((s.width(), s.height()))
            .or_default()
            .push(i as u32);
    }

    // Visit symbols in order of frequency (most common first) so the
    // representative chosen tends to be the most commonly used glyph.
    let mut freq = vec![0u32; identity.symbols.len()];
    for &s in &identity.instance_symbol {
        freq[s as usize] += 1;
    }
    let mut by_freq: Vec<u32> = (0..identity.symbols.len() as u32).collect();
    by_freq.sort_by_key(|&i| std::cmp::Reverse(freq[i as usize]));

    // For each symbol (in descending frequency order), if it's already
    // merged into something else we skip it; otherwise we try to merge in
    // any remaining symbol in the same (w,h), (w±1,h), (w,h±1), (w±1,h±1)
    // buckets that scores within threshold.
    let n = identity.symbols.len();
    let mut remap = (0..n as u32).collect::<Vec<_>>();
    let mut merged = vec![false; n];
    let mut merge_count = 0u32;
    for &rep in &by_freq {
        if merged[rep as usize] {
            continue;
        }
        let rep_bm = &identity.symbols[rep as usize];
        let (rw, rh) = (rep_bm.width(), rep_bm.height());
        for &dw in &[0i32, -1, 1] {
            for &dh in &[0i32, -1, 1] {
                let w = rw as i32 + dw;
                let h = rh as i32 + dh;
                if w < 1 || h < 1 {
                    continue;
                }
                if let Some(candidates) = buckets.get(&(w as u32, h as u32)) {
                    for &cand in candidates {
                        if cand == rep || merged[cand as usize] {
                            continue;
                        }
                        let cand_bm = &identity.symbols[cand as usize];
                        if wxor_distance(rep_bm, cand_bm) <= max_disagreement {
                            remap[cand as usize] = rep;
                            merged[cand as usize] = true;
                            merge_count += 1;
                        }
                    }
                }
            }
        }
    }

    // Build the output symbol list from unmerged reps in original order.
    let mut new_idx = vec![u32::MAX; n];
    let mut new_symbols = Vec::new();
    for i in 0..n {
        if !merged[i] {
            new_idx[i] = new_symbols.len() as u32;
            new_symbols.push(identity.symbols[i].clone());
        }
    }
    // Route merged entries to their representative's new index.
    for i in 0..n {
        if merged[i] {
            new_idx[i] = new_idx[remap[i] as usize];
        }
    }
    let instance_symbol: Vec<u32> = identity
        .instance_symbol
        .iter()
        .map(|&s| new_idx[s as usize])
        .collect();

    LossyClassification {
        symbols: new_symbols,
        instance_symbol,
        lossy_merge_count: merge_count,
    }
}

/// Normalised weighted-XOR distance. Accepts dissimilar sizes by padding
/// the smaller bitmap with zeros to the larger common box. Output is in
/// `[0.0, 2.0]` (roughly — bounded by `xor_popcount / min_area`).
pub fn wxor_distance(a: &Bitmap, b: &Bitmap) -> f32 {
    let aw = a.width().max(b.width());
    let ah = a.height().max(b.height());
    let (ofx_a, ofy_a) = centre_offset(a, aw, ah);
    let (ofx_b, ofy_b) = centre_offset(b, aw, ah);
    let mut xor_count = 0u64;
    let mut a_count = 0u64;
    let mut b_count = 0u64;
    for y in 0..ah as i32 {
        for x in 0..aw as i32 {
            let pa = a.get_pixel(x - ofx_a, y - ofy_a);
            let pb = b.get_pixel(x - ofx_b, y - ofy_b);
            if pa != pb {
                xor_count += 1;
            }
            if pa != 0 {
                a_count += 1;
            }
            if pb != 0 {
                b_count += 1;
            }
        }
    }
    let min_count = a_count.min(b_count).max(1) as f32;
    xor_count as f32 / min_count
}

fn centre_offset(bm: &Bitmap, target_w: u32, target_h: u32) -> (i32, i32) {
    let dx = (target_w as i32 - bm.width() as i32) / 2;
    let dy = (target_h as i32 - bm.height() as i32) / 2;
    (dx, dy)
}

// Helper so the two classifiers can share a single return shape.
impl IdentityClassification {
    fn from_classify(other: Self) -> Self {
        other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::Bitmap;
    use crate::symbol::cc::Component;

    fn mkbm(w: u32, h: u32, pixels: &[(i32, i32)]) -> Bitmap {
        let mut bm = Bitmap::new(w, h).unwrap();
        for &(x, y) in pixels {
            bm.set_pixel(x, y, 1);
        }
        bm
    }
    fn comp(bm: Bitmap, x: u32, y: u32) -> Component {
        Component { x, y, bitmap: bm }
    }

    #[test]
    fn identity_only_when_threshold_one() {
        let a = mkbm(5, 5, &[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]);
        let b = mkbm(5, 5, &[(0, 0), (1, 1), (2, 2), (3, 4), (4, 3)]);
        let comps = vec![comp(a, 0, 0), comp(b, 10, 0)];
        let cls = classify_lossy(&comps, 1.0);
        assert_eq!(cls.symbols.len(), 2);
        assert_eq!(cls.lossy_merge_count, 0);
    }

    #[test]
    fn near_identical_glyphs_merge() {
        // a has 5 set pixels, b differs in 2 pixels. XOR count / min_count
        // = 2/5 = 0.4 → threshold 0.5 allows up to 0.5 disagreement.
        let a = mkbm(5, 5, &[(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]);
        let b = mkbm(5, 5, &[(0, 0), (1, 1), (2, 2), (3, 3), (4, 3)]);
        let comps = vec![comp(a, 0, 0), comp(b.clone(), 10, 0), comp(b, 20, 0)];
        let cls = classify_lossy(&comps, 0.5);
        assert_eq!(cls.symbols.len(), 1);
        assert_eq!(cls.lossy_merge_count, 1);
        assert_eq!(cls.instance_symbol, vec![0, 0, 0]);
    }

    #[test]
    fn dissimilar_glyphs_do_not_merge() {
        let a = mkbm(5, 5, &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
        let b = mkbm(5, 5, &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)]);
        let comps = vec![comp(a, 0, 0), comp(b, 10, 0)];
        let cls = classify_lossy(&comps, 0.9);
        assert_eq!(cls.symbols.len(), 2);
        assert_eq!(cls.lossy_merge_count, 0);
    }
}
