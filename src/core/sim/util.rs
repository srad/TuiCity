use crate::core::map::Map;

/// Iterates every in-bounds tile within `radius` tiles of `(cx, cy)` (inclusive, circular)
/// and calls `f(nx, ny, idx, falloff)` where:
/// - `(nx, ny)` is the target tile position
/// - `idx` is its flat index in `map.overlays` / `map.tiles`
/// - `falloff` is 1.0 at the centre, 0.0 at the edge
///
/// Tiles exactly at the edge (`dist == radius`) receive `falloff = 0.0` and are
/// intentionally included so callers can handle the boundary explicitly.
pub fn for_each_in_radius(
    map: &Map,
    cx: usize,
    cy: usize,
    radius: i32,
    mut f: impl FnMut(usize, usize, usize, f32),
) {
    let radius_sq = (radius * radius) as f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let dist_sq = (dx * dx + dy * dy) as f32;
            if dist_sq > radius_sq {
                continue;
            }
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if !map.in_bounds(nx, ny) {
                continue;
            }
            let falloff = 1.0 - (dist_sq / radius_sq);
            let idx = ny as usize * map.width + nx as usize;
            f(nx as usize, ny as usize, idx, falloff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::Map;

    #[test]
    fn for_each_in_radius_centre_has_falloff_one() {
        let map = Map::new(5, 5);
        let mut centre_falloff = f32::NAN;
        for_each_in_radius(&map, 2, 2, 2, |nx, ny, _idx, falloff| {
            if nx == 2 && ny == 2 {
                centre_falloff = falloff;
            }
        });
        assert!(
            (centre_falloff - 1.0).abs() < 1e-6,
            "centre tile falloff should be 1.0, got {centre_falloff}"
        );
    }

    #[test]
    fn for_each_in_radius_edge_has_falloff_zero() {
        // A tile exactly `radius` tiles away in a cardinal direction sits on the edge.
        let map = Map::new(10, 10);
        let cx = 5usize;
        let cy = 5usize;
        let radius = 3i32;
        let mut edge_falloff: Option<f32> = None;
        for_each_in_radius(&map, cx, cy, radius, |nx, ny, _idx, falloff| {
            // The tile directly `radius` steps north is exactly at distance `radius`.
            if nx == cx && ny == cy - radius as usize {
                edge_falloff = Some(falloff);
            }
        });
        let falloff = edge_falloff.expect("edge tile was not visited");
        assert!(
            falloff.abs() < 1e-6,
            "edge tile falloff should be 0.0, got {falloff}"
        );
    }

    #[test]
    fn for_each_in_radius_visits_all_tiles_within_radius_1() {
        // For radius 1 on a sufficiently large map the 4 cardinal neighbours + centre = 5 tiles.
        // (Diagonal corners have dist_sq = 2 > 1, so they are excluded.)
        let map = Map::new(5, 5);
        let mut count = 0usize;
        for_each_in_radius(&map, 2, 2, 1, |_nx, _ny, _idx, _falloff| {
            count += 1;
        });
        assert_eq!(count, 5, "radius 1 should visit centre + 4 cardinals");
    }

    #[test]
    fn for_each_in_radius_clips_at_map_bounds() {
        // Centre at (0,0): only tiles with x>=0 and y>=0 are valid.
        let map = Map::new(5, 5);
        let mut count = 0usize;
        for_each_in_radius(&map, 0, 0, 2, |_nx, _ny, _idx, _falloff| {
            count += 1;
        });
        // All visits must be in-bounds and falloff must be in [0, 1].
        let mut all_valid = true;
        for_each_in_radius(&map, 0, 0, 2, |nx, ny, _idx, falloff| {
            if !map.in_bounds(nx as i32, ny as i32) || !(0.0..=1.0).contains(&falloff) {
                all_valid = false;
            }
        });
        assert!(all_valid, "all visited tiles must be in-bounds with falloff in [0,1]");
        assert!(count > 0, "should visit at least the centre tile");
    }

    #[test]
    fn for_each_in_radius_idx_matches_position() {
        // Verify that idx == ny * width + nx for every visited tile.
        let map = Map::new(8, 8);
        let mut mismatch = false;
        for_each_in_radius(&map, 4, 4, 3, |nx, ny, idx, _falloff| {
            if idx != ny * map.width + nx {
                mismatch = true;
            }
        });
        assert!(!mismatch, "idx must equal ny * map.width + nx for all visited tiles");
    }
}
