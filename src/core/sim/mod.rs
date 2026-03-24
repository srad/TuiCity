pub mod constants;
pub mod economy;
pub mod growth;
pub mod system;
pub mod systems;
pub mod transport;
pub mod util;

pub use economy::{TaxRates, TaxSector};
use std::collections::{HashMap, VecDeque};

// ── MaintenanceBreakdown ──────────────────────────────────────────────────────

/// Per-category annual maintenance costs, populated by `FinanceSystem` each month.
#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaintenanceBreakdown {
    pub roads: i64,
    pub power_lines: i64,
    pub power_plants: i64,
    pub police: i64,
    pub fire: i64,
    pub parks: i64,
    pub residential_tax: i64,
    pub commercial_tax: i64,
    pub industrial_tax: i64,
    pub total: i64,
    pub annual_tax: i64,
}

// ── Disaster configuration ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DisasterConfig {
    pub fire_enabled: bool,
    pub flood_enabled: bool,
    pub tornado_enabled: bool,
}

impl Default for DisasterConfig {
    fn default() -> Self {
        Self {
            fire_enabled: true,
            flood_enabled: false,
            tornado_enabled: false,
        }
    }
}

// ── Power Plant State ────────────────────────────────────────────────────────

fn plant_efficiency_default() -> f32 {
    1.0
}

fn plant_footprint_default() -> u8 {
    4
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlantState {
    pub age_months: u32,
    pub max_life_months: u32,
    pub capacity_mw: u32,
    /// 1.0 = full output, 0.0 = no output. Degrades to 0 over the last 12 months of life.
    #[serde(default = "plant_efficiency_default")]
    pub efficiency: f32,
    /// Side length of the square footprint in tiles (4 for coal/gas/nuclear, 1 for wind).
    #[serde(default = "plant_footprint_default")]
    pub footprint: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DepotState {
    pub trips_used: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum UnlockMode {
    #[default]
    Historical,
    Sandbox,
}

// ── coord_map_serde modules ──────────────────────────────────────────────────
//
// Serialize/deserialize HashMap<(usize, usize), V> as a JSON array of
// {x, y, state} objects. Both plants and depots use this pattern.

mod plant_map_serde {
    use super::PlantState;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct PlantEntry {
        x: usize,
        y: usize,
        state: PlantState,
    }

    pub fn serialize<S>(
        map: &HashMap<(usize, usize), PlantState>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<PlantEntry> = map
            .iter()
            .map(|(&(x, y), state)| PlantEntry {
                x,
                y,
                state: state.clone(),
            })
            .collect();
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<(usize, usize), PlantState>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<PlantEntry>::deserialize(deserializer)?;
        Ok(entries
            .into_iter()
            .map(|entry| ((entry.x, entry.y), entry.state))
            .collect())
    }
}

mod depot_map_serde {
    use super::DepotState;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct DepotEntry {
        x: usize,
        y: usize,
        state: DepotState,
    }

    pub fn serialize<S>(
        map: &HashMap<(usize, usize), DepotState>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<DepotEntry> = map
            .iter()
            .map(|(&(x, y), state)| DepotEntry {
                x,
                y,
                state: state.clone(),
            })
            .collect();
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<(usize, usize), DepotState>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<DepotEntry>::deserialize(deserializer)?;
        Ok(entries
            .into_iter()
            .map(|entry| ((entry.x, entry.y), entry.state))
            .collect())
    }
}

// ── Sub-structs ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EconomyState {
    pub treasury: i64,
    pub tax_rates: TaxRates,
    pub last_income: i64,
    pub last_breakdown: MaintenanceBreakdown,
    pub unlock_mode: UnlockMode,
}

impl Default for EconomyState {
    fn default() -> Self {
        Self {
            treasury: 20_000,
            tax_rates: TaxRates::default(),
            last_income: 0,
            last_breakdown: MaintenanceBreakdown::default(),
            unlock_mode: UnlockMode::default(),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PopulationState {
    pub population: u64,
    pub residential_population: u64,
    pub commercial_jobs: u64,
    pub industrial_jobs: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DemandState {
    pub res: f32,
    pub comm: f32,
    pub ind: f32,
}

impl Default for DemandState {
    fn default() -> Self {
        Self {
            res: 0.8,
            comm: 0.5,
            ind: 0.4,
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UtilityState {
    pub power_produced_mw: u32,
    pub power_consumed_mw: u32,
    pub water_produced_units: u32,
    pub water_consumed_units: u32,
    /// Total raw emission strength summed across all polluter tiles this tick.
    #[serde(default)]
    pub pollution_emitted: u32,
    /// Total absorption capacity summed across all cleaner tiles this tick.
    #[serde(default)]
    pub pollution_absorbed: u32,
}

#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TripStats {
    pub attempts: u32,
    pub successes: u32,
    pub failures: u32,
    pub road_share: u32,
    pub bus_share: u32,
    pub rail_share: u32,
    pub subway_share: u32,
}

fn default_rng_seed() -> u64 {
    // Fixed non-zero seed so deterministic tests and fresh saves start from a stable baseline.
    0x00C0_FFEE_D15E_A5E5
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RngState {
    #[serde(default = "default_rng_seed")]
    pub transport: u64,
    #[serde(default = "default_rng_seed")]
    pub disaster: u64,
    #[serde(default = "default_rng_seed")]
    pub growth: u64,
}

impl Default for RngState {
    fn default() -> Self {
        let seed = default_rng_seed();
        Self {
            transport: seed,
            disaster: seed,
            growth: seed,
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HistoryState {
    pub demand_res: VecDeque<f32>,
    pub demand_comm: VecDeque<f32>,
    pub demand_ind: VecDeque<f32>,
    pub treasury: VecDeque<i64>,
    pub population: VecDeque<u64>,
    pub income: VecDeque<i64>,
    pub power_balance: VecDeque<i32>,
}

impl HistoryState {
    /// Push one month's worth of values onto all history ring buffers and trim
    /// each buffer to `HISTORY_LEN` entries. All seven deques are always kept
    /// in sync; a `debug_assert` enforces this so stale deques surface in tests.
    pub fn push(
        &mut self,
        demand_res: f32,
        demand_comm: f32,
        demand_ind: f32,
        treasury: i64,
        population: u64,
        income: i64,
        power_balance: i32,
    ) {
        use crate::core::sim::constants::HISTORY_LEN;
        self.demand_res.push_back(demand_res);
        self.demand_comm.push_back(demand_comm);
        self.demand_ind.push_back(demand_ind);
        self.treasury.push_back(treasury);
        self.population.push_back(population);
        self.income.push_back(income);
        self.power_balance.push_back(power_balance);

        while self.demand_res.len() > HISTORY_LEN {
            self.demand_res.pop_front();
            self.demand_comm.pop_front();
            self.demand_ind.pop_front();
            self.treasury.pop_front();
            self.population.pop_front();
            self.income.pop_front();
            self.power_balance.pop_front();
        }

        debug_assert!(
            self.demand_res.len() == self.demand_comm.len()
                && self.demand_res.len() == self.demand_ind.len()
                && self.demand_res.len() == self.treasury.len()
                && self.demand_res.len() == self.population.len()
                && self.demand_res.len() == self.income.len()
                && self.demand_res.len() == self.power_balance.len(),
            "history ring buffers are out of sync"
        );
    }
}

// ── SimState ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    // ── Identity / time ───────────────────────────────────────────────────────
    pub city_name: String,
    pub year: i32,
    pub month: u8,

    // ── Grouped state ─────────────────────────────────────────────────────────
    #[serde(default)]
    pub economy: EconomyState,
    #[serde(default)]
    pub pop: PopulationState,
    #[serde(default)]
    pub demand: DemandState,
    #[serde(default)]
    pub utilities: UtilityState,
    #[serde(default)]
    pub trips: TripStats,
    #[serde(default)]
    pub rng: RngState,
    #[serde(default)]
    pub history: HistoryState,

    // ── Config ────────────────────────────────────────────────────────────────
    #[serde(default)]
    pub disasters: DisasterConfig,

    // ── Placed structures ─────────────────────────────────────────────────────
    #[serde(default, with = "plant_map_serde")]
    pub plants: HashMap<(usize, usize), PlantState>,
    #[serde(default, with = "depot_map_serde")]
    pub depots: HashMap<(usize, usize), DepotState>,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            city_name: "Unnamed City".to_string(),
            year: 1900,
            month: 1,
            economy: EconomyState::default(),
            pop: PopulationState::default(),
            demand: DemandState::default(),
            utilities: UtilityState::default(),
            trips: TripStats::default(),
            rng: RngState::default(),
            history: HistoryState::default(),
            disasters: DisasterConfig::default(),
            plants: HashMap::new(),
            depots: HashMap::new(),
        }
    }
}

impl SimState {
    pub fn month_name(&self) -> &'static str {
        match self.month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintenance_breakdown_defaults_to_zero() {
        let b = MaintenanceBreakdown::default();
        assert_eq!(b.roads, 0);
        assert_eq!(b.total, 0);
        assert_eq!(b.annual_tax, 0);
    }

    #[test]
    fn sim_state_default_has_zero_breakdown() {
        let s = SimState::default();
        assert_eq!(s.economy.last_breakdown.total, 0);
        assert_eq!(s.economy.last_breakdown.annual_tax, 0);
        assert_eq!(s.economy.tax_rates.residential, 9);
    }

    #[test]
    fn push_history_appends_values_to_all_deques() {
        let mut s = SimState::default();
        s.history.push(0.5, 0.3, 0.2, 1000, 500, 100, 50);
        assert_eq!(s.history.demand_res.back(), Some(&0.5));
        assert_eq!(s.history.demand_comm.back(), Some(&0.3));
        assert_eq!(s.history.demand_ind.back(), Some(&0.2));
        assert_eq!(s.history.treasury.back(), Some(&1000));
        assert_eq!(s.history.population.back(), Some(&500));
        assert_eq!(s.history.income.back(), Some(&100));
        assert_eq!(s.history.power_balance.back(), Some(&50));
    }

    #[test]
    fn push_history_trims_to_history_len() {
        use crate::core::sim::constants::HISTORY_LEN;
        let mut s = SimState::default();
        for i in 0..(HISTORY_LEN + 5) {
            s.history.push(i as f32, 0.0, 0.0, 0, 0, 0, 0);
        }
        assert_eq!(s.history.demand_res.len(), HISTORY_LEN);
        assert_eq!(s.history.treasury.len(), HISTORY_LEN);
    }

    #[test]
    fn push_history_all_deques_stay_in_sync() {
        let mut s = SimState::default();
        for _ in 0..30 {
            s.history.push(1.0, 0.5, 0.0, 0, 0, 0, 0);
        }
        let len = s.history.demand_res.len();
        assert_eq!(s.history.demand_comm.len(), len);
        assert_eq!(s.history.demand_ind.len(), len);
        assert_eq!(s.history.treasury.len(), len);
        assert_eq!(s.history.population.len(), len);
        assert_eq!(s.history.income.len(), len);
        assert_eq!(s.history.power_balance.len(), len);
    }

    #[test]
    fn push_history_oldest_value_is_evicted_first() {
        use crate::core::sim::constants::HISTORY_LEN;
        let mut s = SimState::default();
        for i in 0..HISTORY_LEN {
            s.history.push(i as f32, 0.0, 0.0, 0, 0, 0, 0);
        }
        assert_eq!(s.history.demand_res.front(), Some(&0.0));
        s.history.push(99.0, 0.0, 0.0, 0, 0, 0, 0);
        assert_eq!(
            s.history.demand_res.front(),
            Some(&1.0),
            "oldest entry should have been evicted"
        );
        assert_eq!(s.history.demand_res.back(), Some(&99.0));
    }
}
