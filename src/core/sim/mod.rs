pub mod growth;
pub mod system;
pub mod systems;

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

// ── SimState ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub treasury: i64,
    pub population: u64,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
    pub tax_rate: u8,
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
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            city_name: "Unnamed City".to_string(),
            year: 1900,
            month: 1,
            treasury: 20_000,
            population: 0,
            demand_res: 0.8,
            demand_comm: 0.5,
            demand_ind: 0.4,
            tax_rate: 9, // 9% is the neutral tax rate in SC2000
            demand_history_res: Vec::new(),
            demand_history_comm: Vec::new(),
            demand_history_ind: Vec::new(),
            treasury_history: Vec::new(),
            disasters: DisasterConfig::default(),
            last_income: 0,
            last_breakdown: MaintenanceBreakdown::default(),
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
    }
}

