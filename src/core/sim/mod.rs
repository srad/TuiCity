pub mod economy;
pub mod growth;
pub mod system;
pub mod systems;

pub use economy::{TaxRates, TaxSector};

// ── MaintenanceBreakdown ──────────────────────────────────────────────────────

/// Per-category annual maintenance costs, populated by `FinanceSystem` each month.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MaintenanceBreakdown {
    pub roads:        i64,
    pub power_lines:  i64,
    pub power_plants: i64,
    pub police:       i64,
    pub fire:         i64,
    pub parks:        i64,
    pub residential_tax: i64,
    pub commercial_tax:  i64,
    pub industrial_tax:  i64,
    pub total:        i64,
    pub annual_tax:   i64,
}

// ── Disaster configuration ────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisasterConfig {
    pub fire_enabled:    bool,
    pub flood_enabled:   bool,
    pub tornado_enabled: bool,
}

impl Default for DisasterConfig {
    fn default() -> Self {
        Self {
            fire_enabled:    true,
            flood_enabled:   false,
            tornado_enabled: false,
        }
    }
}

// ── Power Plant State ────────────────────────────────────────────────────────

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlantState {
    pub age_months: u32,
    pub max_life_months: u32,
    pub capacity_mw: u32,
}

// ── SimState ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub demand_history_res: Vec<f32>,
    pub demand_history_comm: Vec<f32>,
    pub demand_history_ind: Vec<f32>,
    pub treasury_history: Vec<i64>,
    #[serde(default)]
    pub disasters: DisasterConfig,
    #[serde(default)]
    pub last_income: i64,   // annualised net income (taxes - maintenance×12), updated each month
    #[serde(default)]
    pub last_breakdown: MaintenanceBreakdown,
    
    // New Power System fields
    #[serde(default)]
    pub power_produced_mw: u32,
    #[serde(default)]
    pub power_consumed_mw: u32,
    #[serde(default)]
    pub plants: std::collections::HashMap<(usize, usize), PlantState>,
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
            demand_history_res: Vec::new(),
            demand_history_comm: Vec::new(),
            demand_history_ind: Vec::new(),
            treasury_history: Vec::new(),
            disasters: DisasterConfig::default(),
            last_income: 0,
            last_breakdown: MaintenanceBreakdown::default(),
            power_produced_mw: 0,
            power_consumed_mw: 0,
            plants: std::collections::HashMap::new(),
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
        assert_eq!(s.last_breakdown.total, 0);
        assert_eq!(s.last_breakdown.annual_tax, 0);
        assert_eq!(s.tax_rates.residential, 9);
    }
}
