use crate::core::map::Map;
use crate::core::sim::system::SimSystem;
use crate::core::sim::{growth, SimState};

// ── GrowthSystem ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GrowthSystem;
impl SimSystem for GrowthSystem {
    fn name(&self) -> &str {
        "Growth"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        growth::tick_growth(map, sim);
    }
}
