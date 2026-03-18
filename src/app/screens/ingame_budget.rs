use crate::{app::input::Action, core::{engine::EngineCommand, sim::TaxSector}};

use super::{AppContext, InGameScreen};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BudgetFocus {
    ResidentialTax,
    CommercialTax,
    IndustrialTax,
}

impl BudgetFocus {
    const ALL: [BudgetFocus; 3] = [
        BudgetFocus::ResidentialTax,
        BudgetFocus::CommercialTax,
        BudgetFocus::IndustrialTax,
    ];

    pub fn next(self) -> Self {
        crate::ui::runtime::cycle_next(self, &Self::ALL)
    }

    pub fn prev(self) -> Self {
        crate::ui::runtime::cycle_prev(self, &Self::ALL)
    }

    pub fn tax_sector(self) -> TaxSector {
        match self {
            BudgetFocus::ResidentialTax => TaxSector::Residential,
            BudgetFocus::CommercialTax => TaxSector::Commercial,
            BudgetFocus::IndustrialTax => TaxSector::Industrial,
        }
    }
}

pub struct BudgetUiState {
    pub focused: BudgetFocus,
    pub residential_tax: rat_widget::slider::SliderState<usize>,
    pub commercial_tax: rat_widget::slider::SliderState<usize>,
    pub industrial_tax: rat_widget::slider::SliderState<usize>,
    pub residential_tax_input: rat_widget::text_input::TextInputState,
    pub commercial_tax_input: rat_widget::text_input::TextInputState,
    pub industrial_tax_input: rat_widget::text_input::TextInputState,
}

impl BudgetUiState {
    pub fn new() -> Self {
        let mut residential_tax = rat_widget::slider::SliderState::new_range((0, 100), 1);
        residential_tax.set_value(9);
        let mut commercial_tax = rat_widget::slider::SliderState::new_range((0, 100), 1);
        commercial_tax.set_value(9);
        let mut industrial_tax = rat_widget::slider::SliderState::new_range((0, 100), 1);
        industrial_tax.set_value(9);
        let mut residential_tax_input = rat_widget::text_input::TextInputState::default();
        residential_tax_input.set_text("9");
        let mut commercial_tax_input = rat_widget::text_input::TextInputState::default();
        commercial_tax_input.set_text("9");
        let mut industrial_tax_input = rat_widget::text_input::TextInputState::default();
        industrial_tax_input.set_text("9");

        Self {
            focused: BudgetFocus::ResidentialTax,
            residential_tax,
            commercial_tax,
            industrial_tax,
            residential_tax_input,
            commercial_tax_input,
            industrial_tax_input,
        }
    }
}

impl InGameScreen {
    pub fn sync_budget_tax_from_sim(&mut self, context: &AppContext) {
        let tax_rates = context.engine.read().unwrap().sim.tax_rates;
        self.set_budget_tax_ui_value(TaxSector::Residential, tax_rates.residential as usize);
        self.set_budget_tax_ui_value(TaxSector::Commercial, tax_rates.commercial as usize);
        self.set_budget_tax_ui_value(TaxSector::Industrial, tax_rates.industrial as usize);
    }

    pub fn open_budget(&mut self, context: &AppContext) {
        self.is_budget_open = true;
        self.budget_needs_center = true;
        self.budget_ui.focused = BudgetFocus::ResidentialTax;
        self.sync_budget_tax_from_sim(context);
    }

    pub fn close_budget(&mut self) {
        self.restore_budget_tax_input_if_empty(TaxSector::Residential);
        self.restore_budget_tax_input_if_empty(TaxSector::Commercial);
        self.restore_budget_tax_input_if_empty(TaxSector::Industrial);
        self.is_budget_open = false;
        self.budget_needs_center = false;
        if matches!(self.window_drag, Some(crate::app::WindowDrag::Budget(_, _))) {
            self.window_drag = None;
        }
    }

    pub fn budget_tax_input_mut(
        &mut self,
        sector: TaxSector,
    ) -> &mut rat_widget::text_input::TextInputState {
        match sector {
            TaxSector::Residential => &mut self.budget_ui.residential_tax_input,
            TaxSector::Commercial => &mut self.budget_ui.commercial_tax_input,
            TaxSector::Industrial => &mut self.budget_ui.industrial_tax_input,
        }
    }

    pub fn set_budget_tax_ui_value(&mut self, sector: TaxSector, rate: usize) {
        let rate = rate.min(100);
        match sector {
            TaxSector::Residential => {
                self.budget_ui.residential_tax.set_value(rate);
            }
            TaxSector::Commercial => {
                self.budget_ui.commercial_tax.set_value(rate);
            }
            TaxSector::Industrial => {
                self.budget_ui.industrial_tax.set_value(rate);
            }
        }

        let state = self.budget_tax_input_mut(sector);
        let text = rate.to_string();
        if state.text() != text {
            state.set_text(text);
        }
        state.set_cursor(state.len(), false);
        state.set_invalid(false);
    }

    pub fn set_budget_tax_rate(&mut self, sector: TaxSector, rate: usize, context: &AppContext) {
        let rate = rate.min(100);
        self.set_budget_tax_ui_value(sector, rate);
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::SetTaxRate { sector, rate: rate as u8 });
        }
    }

    pub fn budget_slider_value(&self, sector: TaxSector) -> usize {
        match sector {
            TaxSector::Residential => self.budget_ui.residential_tax.value(),
            TaxSector::Commercial => self.budget_ui.commercial_tax.value(),
            TaxSector::Industrial => self.budget_ui.industrial_tax.value(),
        }
    }

    pub fn adjust_budget_tax(&mut self, sector: TaxSector, delta: i32, context: &AppContext) {
        let current = self.budget_slider_value(sector) as i32;
        let next = (current + delta).clamp(0, 100) as usize;
        self.set_budget_tax_rate(sector, next, context);
    }

    pub fn apply_budget_tax_input(&mut self, sector: TaxSector, context: &AppContext) {
        let state = self.budget_tax_input_mut(sector);
        let mut digits = state
            .text()
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .collect::<String>();

        if digits.len() > 3 {
            digits.truncate(3);
        }

        if digits.is_empty() {
            if state.text() != digits {
                state.set_text(digits);
            }
            state.set_invalid(false);
            state.set_cursor(state.len(), false);
            return;
        }

        let rate = digits.parse::<usize>().unwrap_or(0).min(100);
        let normalized = rate.to_string();
        if digits != normalized {
            digits = normalized;
        }

        if state.text() != digits {
            state.set_text(digits);
        }
        state.set_invalid(false);
        state.set_cursor(state.len(), false);
        self.set_budget_tax_rate(sector, rate, context);
    }

    pub fn restore_budget_tax_input_if_empty(&mut self, sector: TaxSector) {
        if self.budget_tax_input_mut(sector).text().is_empty() {
            let value = self.budget_slider_value(sector);
            self.set_budget_tax_ui_value(sector, value);
        }
    }

    pub fn focus_budget_control_at(&mut self, col: u16, row: u16) {
        if Self::rect_contains(self.budget_ui.residential_tax_input.area, col, row)
            || Self::rect_contains(self.budget_ui.residential_tax.area, col, row)
        {
            self.budget_ui.focused = BudgetFocus::ResidentialTax;
        } else if Self::rect_contains(self.budget_ui.commercial_tax_input.area, col, row)
            || Self::rect_contains(self.budget_ui.commercial_tax.area, col, row)
        {
            self.budget_ui.focused = BudgetFocus::CommercialTax;
        } else if Self::rect_contains(self.budget_ui.industrial_tax_input.area, col, row)
            || Self::rect_contains(self.budget_ui.industrial_tax.area, col, row)
        {
            self.budget_ui.focused = BudgetFocus::IndustrialTax;
        }
    }

    pub fn handle_budget_action(&mut self, action: &Action, context: &AppContext) -> bool {
        if !self.is_budget_open {
            return false;
        }

        match action {
            Action::MenuBack | Action::CharInput('b') | Action::CharInput('B') => {
                self.close_budget();
                true
            }
            Action::MoveCursor(dx, dy) => {
                if *dy != 0 {
                    let sector = self.budget_ui.focused.tax_sector();
                    self.restore_budget_tax_input_if_empty(sector);
                }
                if *dy < 0 {
                    self.budget_ui.focused = self.budget_ui.focused.prev();
                } else if *dy > 0 {
                    self.budget_ui.focused = self.budget_ui.focused.next();
                } else {
                    let sector = self.budget_ui.focused.tax_sector();
                    if *dx < 0 {
                        self.adjust_budget_tax(sector, -1, context);
                    } else if *dx > 0 {
                        self.adjust_budget_tax(sector, 1, context);
                    }
                }
                true
            }
            Action::MenuSelect => true,
            Action::CharInput('$') => {
                self.close_budget();
                true
            }
            Action::MouseDrag { col, row } => {
                if let Some(crate::app::WindowDrag::Budget(ox, oy)) = self.window_drag.as_ref() {
                    self.budget_win.x = col.saturating_sub(*ox);
                    self.budget_win.y = row.saturating_sub(*oy);
                }
                true
            }
            Action::MouseUp { .. } => {
                if matches!(self.window_drag, Some(crate::app::WindowDrag::Budget(_, _))) {
                    self.window_drag = None;
                }
                true
            }
            _ => true,
        }
    }
}
