//! Lossless identity classifier: deduplicates components whose bitmaps are
//! pixel-identical.
//!
//! The classifier hashes every component with [`Bitmap::pixel_hash`] and
//! keeps one representative per hash bucket; hash collisions are resolved
//! by a full pixel-by-pixel comparison. The output is a compact symbol
//! library plus a per-component symbol index.

use std::collections::HashMap;

use crate::bitmap::Bitmap;
use crate::symbol::cc::Component;

/// Output of running the identity classifier on a component list.
#[derive(Clone, Debug)]
pub struct IdentityClassification {
    /// Deduplicated symbol bitmaps in assignment order.
    pub symbols: Vec<Bitmap>,
    /// For each input component (in order), the index into `symbols`.
    pub instance_symbol: Vec<u32>,
}

/// Build a lossless symbol library from a list of components. Components
/// with pixel-identical bitmaps share a symbol index; otherwise a new
/// symbol is appended to the library.
pub fn classify_identity(components: &[Component]) -> IdentityClassification {
    let mut symbols: Vec<Bitmap> = Vec::new();
    let mut instance_symbol = Vec::with_capacity(components.len());
    let mut buckets: HashMap<u64, Vec<u32>> = HashMap::new();

    for c in components {
        let h = c.bitmap.pixel_hash();
        let entry = buckets.entry(h).or_default();
        let mut found: Option<u32> = None;
        for &idx in entry.iter() {
            if symbols[idx as usize] == c.bitmap {
                found = Some(idx);
                break;
            }
        }
        let sym_idx = match found {
            Some(idx) => idx,
            None => {
                let idx = symbols.len() as u32;
                symbols.push(c.bitmap.clone());
                entry.push(idx);
                idx
            }
        };
        instance_symbol.push(sym_idx);
    }

    IdentityClassification {
        symbols,
        instance_symbol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::Bitmap;
    use crate::symbol::cc::Component;

    fn comp_from(bm: Bitmap, x: u32, y: u32) -> Component {
        Component { x, y, bitmap: bm }
    }

    fn boxed_bitmap(w: u32, h: u32) -> Bitmap {
        Bitmap::filled(w, h, 1).unwrap()
    }

    #[test]
    fn unique_and_duplicate_symbols() {
        let a = boxed_bitmap(5, 7);
        let b = boxed_bitmap(4, 4);
        let components = vec![
            comp_from(a.clone(), 0, 0),
            comp_from(b.clone(), 10, 0),
            comp_from(a.clone(), 20, 0),
            comp_from(a.clone(), 30, 0),
            comp_from(b.clone(), 40, 0),
        ];
        let cls = classify_identity(&components);
        assert_eq!(cls.symbols.len(), 2);
        assert_eq!(cls.instance_symbol, vec![0, 1, 0, 0, 1]);
    }
}
