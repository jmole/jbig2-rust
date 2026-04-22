//! Connected-component extraction for bi-level bitmaps.
//!
//! The extractor walks every foreground pixel, runs a stack-based flood fill
//! (8-connected by default), and emits a compact per-component record:
//! bounding box on the source page plus a cropped bitmap of just that
//! component's pixels.

use crate::bitmap::Bitmap;

/// A single connected component found on a page.
#[derive(Clone, Debug)]
pub struct Component {
    /// Top-left X on the source page.
    pub x: u32,
    /// Top-left Y on the source page.
    pub y: u32,
    /// Tight bitmap of just this component (size = bounding box).
    pub bitmap: Bitmap,
}

impl Component {
    /// Width of the component bitmap.
    pub fn width(&self) -> u32 {
        self.bitmap.width()
    }
    /// Height of the component bitmap.
    pub fn height(&self) -> u32 {
        self.bitmap.height()
    }
}

/// 8-connected flood-fill connected component extractor. Operates in a
/// reading order sweep: components are emitted roughly in top-to-bottom,
/// left-to-right order of their top-left corner.
pub fn extract_components(page: &Bitmap) -> Vec<Component> {
    let w = page.width();
    let h = page.height();
    if w == 0 || h == 0 {
        return Vec::new();
    }
    // visited[y*w + x] == true iff pixel has already been assigned to a
    // component.
    let total = (w as usize) * (h as usize);
    let mut visited = vec![false; total];
    let mut out = Vec::new();

    for seed_y in 0..h as i32 {
        for seed_x in 0..w as i32 {
            let idx = (seed_y as usize) * (w as usize) + seed_x as usize;
            if visited[idx] {
                continue;
            }
            if page.get_pixel(seed_x, seed_y) == 0 {
                visited[idx] = true;
                continue;
            }
            // Flood-fill from (seed_x, seed_y), collecting every foreground
            // neighbour. Track bounding box + the pixel list.
            let mut stack: Vec<(i32, i32)> = Vec::new();
            stack.push((seed_x, seed_y));
            visited[idx] = true;
            let mut pixels: Vec<(i32, i32)> = Vec::new();
            let mut min_x = seed_x;
            let mut max_x = seed_x;
            let mut min_y = seed_y;
            let mut max_y = seed_y;
            while let Some((px, py)) = stack.pop() {
                pixels.push((px, py));
                if px < min_x {
                    min_x = px;
                }
                if px > max_x {
                    max_x = px;
                }
                if py < min_y {
                    min_y = py;
                }
                if py > max_y {
                    max_y = py;
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = px + dx;
                        let ny = py + dy;
                        if nx < 0 || ny < 0 || nx as u32 >= w || ny as u32 >= h {
                            continue;
                        }
                        let nidx = (ny as usize) * (w as usize) + nx as usize;
                        if visited[nidx] {
                            continue;
                        }
                        if page.get_pixel(nx, ny) == 0 {
                            visited[nidx] = true;
                            continue;
                        }
                        visited[nidx] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            let cw = (max_x - min_x + 1) as u32;
            let ch = (max_y - min_y + 1) as u32;
            let mut bm = Bitmap::new(cw, ch).expect("component bbox must be valid");
            for (px, py) in &pixels {
                bm.set_pixel(px - min_x, py - min_y, 1);
            }
            out.push(Component {
                x: min_x as u32,
                y: min_y as u32,
                bitmap: bm,
            });
        }
    }
    out
}

/// Reconstruct a page bitmap from a set of components. Each component is
/// OR-blitted at its stored top-left coordinate.
pub fn composite_components(width: u32, height: u32, components: &[Component]) -> Bitmap {
    let mut page = Bitmap::new(width, height).expect("page dimensions must be valid");
    for c in components {
        for cy in 0..c.height() as i32 {
            for cx in 0..c.width() as i32 {
                if c.bitmap.get_pixel(cx, cy) != 0 {
                    page.set_pixel(c.x as i32 + cx, c.y as i32 + cy, 1);
                }
            }
        }
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_page() {
        let bm = Bitmap::new(20, 10).unwrap();
        let cs = extract_components(&bm);
        assert!(cs.is_empty());
    }

    #[test]
    fn three_disjoint_boxes() {
        let mut bm = Bitmap::new(40, 20).unwrap();
        for y in 2..5 {
            for x in 2..6 {
                bm.set_pixel(x, y, 1);
            }
        }
        for y in 2..8 {
            for x in 20..24 {
                bm.set_pixel(x, y, 1);
            }
        }
        for y in 12..18 {
            for x in 30..38 {
                bm.set_pixel(x, y, 1);
            }
        }
        let cs = extract_components(&bm);
        assert_eq!(cs.len(), 3);
        assert_eq!(cs[0].width(), 4);
        assert_eq!(cs[0].height(), 3);
        assert_eq!(cs[1].width(), 4);
        assert_eq!(cs[1].height(), 6);
        assert_eq!(cs[2].width(), 8);
        assert_eq!(cs[2].height(), 6);
        let rebuilt = composite_components(bm.width(), bm.height(), &cs);
        assert_eq!(rebuilt, bm);
    }

    #[test]
    fn diagonally_connected() {
        // Two pixels at (3,3) and (4,4) are 8-connected but not 4-connected.
        let mut bm = Bitmap::new(10, 10).unwrap();
        bm.set_pixel(3, 3, 1);
        bm.set_pixel(4, 4, 1);
        let cs = extract_components(&bm);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].width(), 2);
        assert_eq!(cs[0].height(), 2);
    }
}
