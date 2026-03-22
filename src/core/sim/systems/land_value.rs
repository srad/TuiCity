use crate::core::map::{Map, Tile};
use crate::core::sim::constants::{
    LV_BASELINE, LV_HOSPITAL_BONUS, LV_HOSPITAL_RADIUS, LV_LIBRARY_BONUS, LV_LIBRARY_RADIUS,
    LV_PARK_BONUS, LV_PARK_RADIUS, LV_POLLUTION_DIVISOR, LV_SCHOOL_BONUS, LV_SCHOOL_RADIUS,
    LV_STADIUM_BONUS, LV_STADIUM_RADIUS, LV_WATER_BONUS, LV_WATER_RADIUS,
};
use crate::core::sim::system::SimSystem;
use crate::core::sim::util::for_each_in_radius;
use crate::core::sim::SimState;

// ── LandValueSystem ───────────────────────────────────────────────────────────

fn apply_proximity_bonus(map: &Map, lv: &mut Vec<u16>, source: Tile, radius: i32, max_bonus: f32) {
    for y in 0..map.height {
        for x in 0..map.width {
            if map.get(x, y) != source {
                continue;
            }
            for_each_in_radius(map, x, y, radius, |_nx, _ny, idx, falloff| {
                lv[idx] = lv[idx].saturating_add((falloff * max_bonus) as u16);
            });
        }
    }
}

#[derive(Debug)]
pub struct LandValueSystem;
impl SimSystem for LandValueSystem {
    fn name(&self) -> &str {
        "LandValue"
    }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        let n = map.width * map.height;
        let mut lv: Vec<u16> = vec![LV_BASELINE; n];

        apply_proximity_bonus(map, &mut lv, Tile::Water, LV_WATER_RADIUS, LV_WATER_BONUS);
        apply_proximity_bonus(map, &mut lv, Tile::Park, LV_PARK_RADIUS, LV_PARK_BONUS);
        apply_proximity_bonus(
            map,
            &mut lv,
            Tile::Hospital,
            LV_HOSPITAL_RADIUS,
            LV_HOSPITAL_BONUS,
        );
        apply_proximity_bonus(map, &mut lv, Tile::School, LV_SCHOOL_RADIUS, LV_SCHOOL_BONUS);
        apply_proximity_bonus(
            map,
            &mut lv,
            Tile::Stadium,
            LV_STADIUM_RADIUS,
            LV_STADIUM_BONUS,
        );
        apply_proximity_bonus(
            map,
            &mut lv,
            Tile::Library,
            LV_LIBRARY_RADIUS,
            LV_LIBRARY_BONUS,
        );

        // Pollution penalty (each point of pollution reduces land value)
        for (i, ov) in map.overlays.iter().enumerate().take(n) {
            let penalty = ov.pollution as u16 / LV_POLLUTION_DIVISOR;
            lv[i] = lv[i].saturating_sub(penalty);
        }

        // Write back (clamped to u8)
        for (i, ov) in map.overlays.iter_mut().enumerate().take(n) {
            ov.land_value = lv[i].min(255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::system::SimSystem;

    fn run_land_value(map: &mut Map) {
        LandValueSystem.tick(map, &mut crate::core::sim::SimState::default());
    }

    #[test]
    fn baseline_land_value_applied_to_empty_map() {
        let mut map = Map::new(3, 3);
        run_land_value(&mut map);
        // Every tile should have at least the baseline land value.
        for ov in &map.overlays {
            assert_eq!(ov.land_value, LV_BASELINE.min(255) as u8);
        }
    }

    #[test]
    fn water_tile_raises_adjacent_land_value_above_baseline() {
        let mut map = Map::new(5, 5);
        map.set(2, 2, Tile::Water);
        run_land_value(&mut map);
        // Tiles immediately adjacent to water should have bonus > 0.
        let adjacent = map.get_overlay(2, 1).land_value;
        assert!(
            adjacent > LV_BASELINE.min(255) as u8,
            "tile adjacent to water should exceed baseline, got {adjacent}"
        );
    }

    #[test]
    fn water_source_tile_itself_gets_full_bonus() {
        // The water tile is itself visited by for_each_in_radius at dist=0, falloff=1.
        let mut map = Map::new(3, 3);
        map.set(1, 1, Tile::Water);
        run_land_value(&mut map);
        let water_lv = map.get_overlay(1, 1).land_value;
        let expected = (LV_BASELINE as f32 + LV_WATER_BONUS).min(255.0) as u8;
        assert_eq!(water_lv, expected);
    }

    #[test]
    fn park_bonus_is_smaller_than_water_bonus_at_equal_distance() {
        let mut map = Map::new(10, 1);
        map.set(0, 0, Tile::Water);
        let mut map2 = Map::new(10, 1);
        map2.set(0, 0, Tile::Park);
        run_land_value(&mut map);
        run_land_value(&mut map2);
        // Water gives up to +40, park up to +30 — so at distance 1 water tile wins.
        let lv_water = map.get_overlay(1, 0).land_value;
        let lv_park = map2.get_overlay(1, 0).land_value;
        assert!(
            lv_water > lv_park,
            "water bonus ({lv_water}) should exceed park bonus ({lv_park}) at same distance"
        );
    }

    #[test]
    fn pollution_reduces_land_value_below_baseline() {
        let mut map = Map::new(1, 1);
        map.overlays[0].pollution = LV_POLLUTION_DIVISOR as u8; // penalty of exactly 1
        run_land_value(&mut map);
        let expected = LV_BASELINE.saturating_sub(1).min(255) as u8;
        assert_eq!(map.overlays[0].land_value, expected);
    }

    #[test]
    fn max_pollution_does_not_underflow() {
        let mut map = Map::new(1, 1);
        map.overlays[0].pollution = 255;
        run_land_value(&mut map); // should not panic
        // Value is >= 0 by construction (saturating_sub)
        let _ = map.overlays[0].land_value;
    }

    #[test]
    fn hospital_raises_adjacent_land_value() {
        let mut map = Map::new(5, 5);
        map.set(2, 2, Tile::Hospital);
        run_land_value(&mut map);
        let adjacent = map.get_overlay(2, 1).land_value;
        assert!(
            adjacent > LV_BASELINE.min(255) as u8,
            "tile adjacent to hospital should exceed baseline, got {adjacent}"
        );
    }

    #[test]
    fn land_value_never_exceeds_255() {
        // Stack water + park + hospital on adjacent tiles.
        let mut map = Map::new(5, 5);
        map.set(1, 2, Tile::Water);
        map.set(3, 2, Tile::Park);
        map.set(2, 1, Tile::Hospital);
        run_land_value(&mut map);
        for ov in &map.overlays {
            assert!(ov.land_value <= 255);
        }
    }
}
