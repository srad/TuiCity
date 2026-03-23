use std::collections::VecDeque;
use std::sync::Arc;

use crate::{
    app::camera::Camera,
    app::save::SaveEntry,
    app::screens::{BudgetFocus, NewCityField},
    core::{
        map::{Map, ViewLayer},
        sim::{MaintenanceBreakdown, SimState, TaxRates},
        tool::{Tool, ToolContext},
    },
    textgen::types::AdvisorDomain,
    ui::{
        runtime::ToolChooserKind,
        theme::{OverlayMode, ThemePreset},
    },
};

#[derive(Clone, Debug)]
pub struct StartViewModel {
    pub selected: usize,
    pub options: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LoadCityViewModel {
    pub saves: Arc<[SaveEntry]>,
    pub selected: usize,
    pub is_loading: bool,
    pub loading_indicator: &'static str,
    pub confirm_dialog: Option<ConfirmDialogViewModel>,
}

#[derive(Clone, Debug)]
pub struct ThemeSettingsViewModel {
    pub themes: Vec<ThemePreset>,
    pub selected: usize,
    pub active: ThemePreset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlmStatus {
    /// LLM feature not compiled in
    Disabled,
    /// Feature compiled but model not found / failed to load
    Unavailable,
    /// Model loaded and worker thread running
    Active,
}

#[derive(Clone, Debug)]
pub struct SettingsViewModel {
    pub options: Vec<String>,
    pub selected: usize,
    pub current_theme_label: String,
    pub llm_status: LlmStatus,
}

#[derive(Clone, Debug)]
pub struct DownloadProgressViewModel {
    pub label: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<u8>,
    pub cancelling: bool,
}

#[derive(Clone, Debug)]
pub struct LlmSetupViewModel {
    pub llm_enabled: bool,
    pub model_installed: bool,
    pub selected_model_label: String,
    pub selected_model_description: String,
    pub selected_model_size_label: String,
    pub gpu_mode_label: String,
    pub gpu_mode_description: String,
    pub backend_status: String,
    pub gpu_status: String,
    pub download_progress: Option<DownloadProgressViewModel>,
    pub download_notice: Option<String>,
    pub download_failed: Option<String>,
    pub selected: usize,
    pub confirm_dialog: Option<ConfirmDialogViewModel>,
}

#[derive(Clone, Debug)]
pub struct NewCityViewModel {
    pub preview_map: Map,
    pub focused_field: NewCityField,
    pub city_name: String,
    pub seed_text: String,
    pub water_pct: usize,
    pub trees_pct: usize,
    pub terrain_brush: Option<crate::app::screens::TerrainBrush>,
    /// Cursor tile position and whether map cursor mode is active.
    pub cursor: (usize, usize),
    pub map_cursor_active: bool,
    /// Whether an LLM city name generation request is in flight.
    pub llm_name_pending: bool,
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
            treasury: sim.economy.treasury,
            current_annual_tax: sim.economy.last_breakdown.annual_tax,
            breakdown: sim.economy.last_breakdown.clone(),
            residential_population: sim.pop.residential_population,
            commercial_jobs: sim.pop.commercial_jobs,
            industrial_jobs: sim.pop.industrial_jobs,
        }
    }
}

#[derive(Clone)]
pub struct ToolbarPaletteViewModel {
    pub current_tool: Tool,
    pub zone_tool: Tool,
    pub transport_tool: Tool,
    pub utility_tool: Tool,
    pub power_plant_tool: Tool,
    pub building_tool: Tool,
    pub terrain_tool: Tool,
    pub chooser: Option<ToolChooserKind>,
    pub view_layer: ViewLayer,
}

#[derive(Clone)]
pub struct ToolChooserViewModel {
    pub selected_tool: Tool,
    pub tools: Vec<Tool>,
    pub ctx: ToolContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmDialogButtonRole {
    Accept,
    Alternate,
    Cancel,
}

#[derive(Clone, Debug)]
pub struct ConfirmDialogButtonViewModel {
    pub label: String,
    pub role: ConfirmDialogButtonRole,
}

#[derive(Clone, Debug)]
pub struct ConfirmDialogViewModel {
    pub title: String,
    pub message: String,
    pub selected: usize,
    pub buttons: Vec<ConfirmDialogButtonViewModel>,
}

impl ConfirmDialogViewModel {
    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }

    pub fn index_for_role(&self, role: ConfirmDialogButtonRole) -> Option<usize> {
        self.buttons.iter().position(|button| button.role == role)
    }

    pub fn selected_role(&self) -> Option<ConfirmDialogButtonRole> {
        self.buttons.get(self.selected).map(|button| button.role)
    }
}

#[derive(Clone)]
pub struct TextWindowViewModel {
    pub lines: Vec<String>,
    pub scroll_y: u16,
}

#[derive(Clone)]
pub struct StatisticsWindowViewModel {
    pub city_name: String,
    pub current_population: u64,
    pub current_treasury: i64,
    pub current_income: i64,
    pub current_power_produced: u32,
    pub current_power_consumed: u32,
    pub treasury_history: VecDeque<i64>,
    pub population_history: VecDeque<u64>,
    pub income_history: VecDeque<i64>,
    pub power_balance_history: VecDeque<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct NewsTickerViewModel {
    pub full_text: String,
    pub scroll_offset: usize,
    pub is_alerting: bool,
}

#[derive(Clone, Debug)]
pub struct AdvisorViewModel {
    pub domain: AdvisorDomain,
    pub text: Option<String>,
    pub pending: bool,
}

#[derive(Clone, Debug)]
pub struct NewspaperSection {
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct NewspaperPage {
    pub title: String,
    pub sections: Vec<NewspaperSection>,
}

#[derive(Clone, Debug)]
pub struct NewspaperViewModel {
    pub pages: Vec<NewspaperPage>,
    pub pending: bool,
    pub city_name: String,
    pub month: u8,
    pub year: i32,
    pub current_page: usize,
    pub selected_section_index: usize,
    /// Which section on the current page is expanded in the detail popup.
    pub detail_section_index: Option<usize>,
}

#[derive(Clone)]
pub struct InGameDesktopView {
    pub map: Map,
    pub sim: SimState,
    pub camera: Camera,
    pub current_tool: Tool,
    pub toolbar: ToolbarPaletteViewModel,
    pub tool_chooser: Option<ToolChooserViewModel>,
    pub confirm_dialog: Option<ConfirmDialogViewModel>,
    pub paused: bool,
    pub overlay_mode: OverlayMode,
    pub view_layer: ViewLayer,
    pub menu_active: bool,
    pub menu_selected: usize,
    pub menu_item_selected: usize,
    pub status_message: Option<String>,
    pub news_ticker: NewsTickerViewModel,
    pub line_preview: Vec<(usize, usize)>,
    pub rect_preview: Vec<(usize, usize)>,
    pub inspect_pos: Option<(usize, usize)>,
    pub budget: BudgetViewModel,
    pub statistics: Option<StatisticsWindowViewModel>,
    pub help: Option<TextWindowViewModel>,
    pub about: Option<TextWindowViewModel>,
    pub legend: Option<TextWindowViewModel>,
    pub advisor: Option<AdvisorViewModel>,
    pub newspaper: Option<NewspaperViewModel>,
}

