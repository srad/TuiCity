pub mod economy;
pub mod growth;
pub mod system;
pub mod systems;
pub mod transport;

pub use economy::{TaxRates, TaxSector};
use std::collections::{HashMap, VecDeque};

fn default_transport_rng_state() -> u64 {
    // Fixed non-zero seed so deterministic tests and legacy saves start from a stable baseline.
    0x00C0_FFEE_D15E_A5E5
}

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

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlantState {
    pub age_months: u32,
    pub max_life_months: u32,
    pub capacity_mw: u32,
    /// 1.0 = full output, 0.0 = no output. Degrades to 0 over the last 12 months of life.
    #[serde(default = "plant_efficiency_default")]
    pub efficiency: f32,
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

// ── SimState ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub treasury: i64,
    pub population: u64,
    pub residential_population: u64,
    pub commercial_jobs: u64,
    pub industrial_jobs: u64,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
    pub tax_rates: TaxRates,
    #[serde(default)]
    pub demand_history_res: VecDeque<f32>,
    #[serde(default)]
    pub demand_history_comm: VecDeque<f32>,
    #[serde(default)]
    pub demand_history_ind: VecDeque<f32>,
    #[serde(default)]
    pub treasury_history: VecDeque<i64>,
    #[serde(default)]
    pub population_history: VecDeque<u64>,
    #[serde(default)]
    pub income_history: VecDeque<i64>,
    #[serde(default)]
    pub power_balance_history: VecDeque<i32>,
    #[serde(default)]
    pub disasters: DisasterConfig,
    #[serde(default)]
    pub last_income: i64, // annualised net income (taxes - maintenance×12), updated each month
    #[serde(default)]
    pub last_breakdown: MaintenanceBreakdown,

    // New Power System fields
    #[serde(default)]
    pub power_produced_mw: u32,
    #[serde(default)]
    pub power_consumed_mw: u32,
    #[serde(default)]
    pub water_produced_units: u32,
    #[serde(default)]
    pub water_consumed_units: u32,
    #[serde(default = "default_transport_rng_state")]
    pub transport_rng_state: u64,
    #[serde(default = "default_transport_rng_state")]
    pub disaster_rng_state: u64,
    #[serde(default = "default_transport_rng_state")]
    pub growth_rng_state: u64,
    // Monthly transport summaries are persisted mainly for UI/debugging and to make save/load
    // roundtrips exact; they are recalculated every simulation tick.
    #[serde(default)]
    pub trip_attempts: u32,
    #[serde(default)]
    pub trip_successes: u32,
    #[serde(default)]
    pub trip_failures: u32,
    #[serde(default)]
    pub road_share: u32,
    #[serde(default)]
    pub bus_share: u32,
    #[serde(default)]
    pub rail_share: u32,
    #[serde(default)]
    pub subway_share: u32,
    #[serde(default)]
    pub unlock_mode: UnlockMode,
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
            treasury: 20_000,
            population: 0,
            residential_population: 0,
            commercial_jobs: 0,
            industrial_jobs: 0,
            demand_res: 0.8,
            demand_comm: 0.5,
            demand_ind: 0.4,
            tax_rates: TaxRates::default(),
            demand_history_res: VecDeque::new(),
            demand_history_comm: VecDeque::new(),
            demand_history_ind: VecDeque::new(),
            treasury_history: VecDeque::new(),
            population_history: VecDeque::new(),
            income_history: VecDeque::new(),
            power_balance_history: VecDeque::new(),
            disasters: DisasterConfig::default(),
            last_income: 0,
            last_breakdown: MaintenanceBreakdown::default(),
            power_produced_mw: 0,
            power_consumed_mw: 0,
            water_produced_units: 0,
            water_consumed_units: 0,
            transport_rng_state: default_transport_rng_state(),
            disaster_rng_state: default_transport_rng_state(),
            growth_rng_state: default_transport_rng_state(),
            trip_attempts: 0,
            trip_successes: 0,
            trip_failures: 0,
            road_share: 0,
            bus_share: 0,
            rail_share: 0,
            subway_share: 0,
            unlock_mode: UnlockMode::default(),
            plants: HashMap::new(),
            depots: HashMap::new(),
        }
    }
}

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
        plants: &HashMap<(usize, usize), PlantState>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<PlantEntry> = plants
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
        depots: &HashMap<(usize, usize), DepotState>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<DepotEntry> = depots
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
        assert_eq!(s.last_breakdown.total, 0);
        assert_eq!(s.last_breakdown.annual_tax, 0);
        assert_eq!(s.tax_rates.residential, 9);
    }
}
