pub mod growth;
pub mod system;
pub mod systems;

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

