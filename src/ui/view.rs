use crate::{
    app::camera::Camera,
    app::save::SaveEntry,
    app::screens::{BudgetFocus, NewCityField},
    core::{
        map::Map,
        sim::{MaintenanceBreakdown, SimState, TaxRates},
        tool::Tool,
    },
    ui::theme::OverlayMode,
};

#[derive(Clone, Debug)]
pub struct StartViewModel {
    pub selected: usize,
    pub options: [&'static str; 3],
}

#[derive(Clone, Debug)]
pub struct LoadCityViewModel {
    pub saves: Vec<SaveEntry>,
    pub selected: usize,
}

#[derive(Clone, Debug)]
pub struct NewCityViewModel {
    pub preview_map: Map,
    pub focused_field: NewCityField,
    pub city_name: String,
    pub seed_text: String,
    pub water_pct: usize,
    pub trees_pct: usize,
}

#[derive(Clone, Debug)]
pub struct BudgetViewModel {
    pub focused: BudgetFocus,
    pub tax_rates: TaxRates,
    pub residential_input: String,
    pub commercial_input: String,
    pub industrial_input: String,
    pub treasury: i64,
    pub current_annual_tax: i64,
    pub breakdown: MaintenanceBreakdown,
    pub residential_population: u64,
    pub commercial_jobs: u64,
    pub industrial_jobs: u64,
}

impl BudgetViewModel {
    pub fn from_sim(
        sim: &SimState,
        focused: BudgetFocus,
        tax_rates: TaxRates,
        residential_input: String,
        commercial_input: String,
        industrial_input: String,
    ) -> Self {
        Self {
            focused,
            tax_rates,
            residential_input,
            commercial_input,
            industrial_input,
            treasury: sim.treasury,
            current_annual_tax: sim.last_breakdown.annual_tax,
            breakdown: sim.last_breakdown.clone(),
            residential_population: sim.residential_population,
            commercial_jobs: sim.commercial_jobs,
            industrial_jobs: sim.industrial_jobs,
        }
    }
}

#[derive(Clone)]
pub struct InGameDesktopView {
    pub map: Map,
    pub sim: SimState,
    pub camera: Camera,
    pub current_tool: Tool,
    pub paused: bool,
    pub overlay_mode: OverlayMode,
    pub menu_active: bool,
    pub menu_selected: usize,
    pub menu_item_selected: usize,
    pub status_message: Option<String>,
    pub line_preview: Vec<(usize, usize)>,
    pub rect_preview: Vec<(usize, usize)>,
    pub inspect_pos: Option<(usize, usize)>,
    pub budget: BudgetViewModel,
}

#[derive(Clone)]
pub enum ScreenView {
    Start(StartViewModel),
    LoadCity(LoadCityViewModel),
    NewCity(NewCityViewModel),
    InGame(InGameDesktopView),
}
