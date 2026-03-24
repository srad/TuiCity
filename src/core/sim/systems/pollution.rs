use crate::core::map::Map;
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── PollutionSystem ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PollutionSystem;
impl SimSystem for PollutionSystem {
    fn name(&self) -> &str {
        "Pollution"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        // Reset
        for o in map.overlays.iter_mut() {
            o.pollution = 0;
        }

        // Collect pollution sources using Tile::pollution_emission()
        let mut sources: Vec<(usize, usize, u8)> = Vec::new();
        let mut total_emitted: u32 = 0;
        for y in 0..map.height {
            for x in 0..map.width {
                let traffic = map.get_overlay(x, y).traffic;
                let strength = map.get(x, y).pollution_emission(traffic);
                if strength > 0 {
                    total_emitted += strength as u32;
                    sources.push((x, y, strength));
                }
            }
        }

        // Radial diffusion with distance falloff
        use crate::core::sim::constants::POLLUTION_RADIUS;
        let radius_sq = (POLLUTION_RADIUS * POLLUTION_RADIUS) as f32;
        for (sx, sy, strength) in sources {
            for dy in -POLLUTION_RADIUS..=POLLUTION_RADIUS {
                for dx in -POLLUTION_RADIUS..=POLLUTION_RADIUS {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    if dist_sq > radius_sq {
                        continue;
                    }
                    let nx = sx as i32 + dx;
                    let ny = sy as i32 + dy;
                    if !map.in_bounds(nx, ny) {
                        continue;
                    }
                    let falloff = 1.0 - (dist_sq / radius_sq);
                    let amount = (strength as f32 * falloff) as u8;
                    let idx = ny as usize * map.width + nx as usize;
                    map.overlays[idx].pollution =
                        map.overlays[idx].pollution.saturating_add(amount);
                }
            }
        }

        // Collect cleaners using Tile::pollution_cleaner()
        let mut cleaners: Vec<(usize, usize, i32, u8)> = Vec::new();
        let mut total_absorbed: u32 = 0;
        for y in 0..map.height {
            for x in 0..map.width {
                if let Some((radius, amount)) = map.get(x, y).pollution_cleaner() {
                    total_absorbed += amount as u32;
                    cleaners.push((x, y, radius, amount));
                }
            }
        }

        // Apply cleaner scrubbing
        for (cx, cy, radius, amount) in cleaners {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if !map.in_bounds(nx, ny) {
                        continue;
                    }
                    let idx = ny as usize * map.width + nx as usize;
                    map.overlays[idx].pollution =
                        map.overlays[idx].pollution.saturating_sub(amount);
                }
            }
        }

        sim.utilities.pollution_emitted = total_emitted;
        sim.utilities.pollution_absorbed = total_absorbed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::Tile;
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::SimState;

    // ── Tile classification ───────────────────────────────────────────────────

    #[test]
    fn polluter_tiles_emit_nonzero() {
        for tile in [
            Tile::IndHeavy,
            Tile::IndLight,
            Tile::PowerPlantCoal,
            Tile::PowerPlantGas,
            Tile::Highway,
        ] {
            assert!(tile.pollution_emission(0) > 0, "{:?} should emit pollution", tile);
        }
    }

    #[test]
    fn road_emission_scales_with_traffic() {
        assert_eq!(Tile::Road.pollution_emission(0), 0);
        assert_eq!(Tile::Road.pollution_emission(40), 10);
        assert_eq!(Tile::RoadPowerLine.pollution_emission(80), 20);
    }

    #[test]
    fn cleaner_tiles_return_some() {
        assert!(Tile::Park.pollution_cleaner().is_some(), "Park should be a cleaner");
        assert!(Tile::Trees.pollution_cleaner().is_some(), "Trees should be a cleaner");
    }

    #[test]
    fn non_participant_tiles_return_zero_and_none() {
        for tile in [Tile::Grass, Tile::Road, Tile::ResLow, Tile::Water, Tile::PowerLine] {
            assert_eq!(tile.pollution_emission(0), 0, "{:?} should not emit", tile);
            assert!(tile.pollution_cleaner().is_none(), "{:?} should not clean", tile);
        }
    }

    // ── Integration ───────────────────────────────────────────────────────────

    #[test]
    fn polluter_raises_overlay_on_same_tile() {
        let mut map = Map::new(3, 3);
        let mut sim = SimState::default();
        map.set(1, 1, Tile::IndHeavy);

        PollutionSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(1, 1).pollution > 0,
            "IndHeavy centre tile should have pollution"
        );
    }

    #[test]
    fn cleaner_reduces_adjacent_pollution() {
        let mut map = Map::new(5, 1);
        let mut sim = SimState::default();
        // Polluter on left, cleaner on right adjacent
        map.set(0, 0, Tile::IndHeavy);
        map.set(1, 0, Tile::Park);

        PollutionSystem.tick(&mut map, &mut sim);

        // Tile 1 (Park) should have less pollution than tile 0 (IndHeavy centre)
        let polluter_level = map.get_overlay(0, 0).pollution;
        let cleaner_level = map.get_overlay(1, 0).pollution;
        assert!(
            cleaner_level < polluter_level,
            "Park tile should have lower pollution than the polluter (got {} vs {})",
            cleaner_level,
            polluter_level
        );
    }

    #[test]
    fn utilities_track_emitted_and_absorbed() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::IndHeavy);
        map.set(2, 0, Tile::Park);

        PollutionSystem.tick(&mut map, &mut sim);

        assert!(sim.utilities.pollution_emitted > 0, "Should track emitted pollution");
        assert!(sim.utilities.pollution_absorbed > 0, "Should track absorbed pollution");
    }
}

