use crate::{app::{input::Action, WindowId}, core::{engine::EngineCommand, sim::TaxSector}};

use super::{AppContext, InGameScreen};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub struct BudgetState {
    pub focused: BudgetFocus,
    pub residential_tax: usize,
    pub commercial_tax: usize,
    pub industrial_tax: usize,
    pub residential_tax_input: String,
    pub commercial_tax_input: String,
    pub industrial_tax_input: String,
}

impl BudgetState {
    pub fn new() -> Self {
        Self {
            focused: BudgetFocus::ResidentialTax,
            residential_tax: 9,
            commercial_tax: 9,
            industrial_tax: 9,
            residential_tax_input: "9".to_string(),
            commercial_tax_input: "9".to_string(),
            industrial_tax_input: "9".to_string(),
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
        self.desktop.open(WindowId::Budget, true);
        self.budget_ui.focused = BudgetFocus::ResidentialTax;
        self.sync_budget_tax_from_sim(context);
    }

    pub fn close_budget(&mut self) {
        self.restore_budget_tax_input_if_empty(TaxSector::Residential);
        self.restore_budget_tax_input_if_empty(TaxSector::Commercial);
        self.restore_budget_tax_input_if_empty(TaxSector::Industrial);
        self.desktop.close(WindowId::Budget);
    }

    pub fn budget_tax_input_mut(
        &mut self,
        sector: TaxSector,
    ) -> &mut String {
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
                self.budget_ui.residential_tax = rate;
            }
            TaxSector::Commercial => {
                self.budget_ui.commercial_tax = rate;
            }
            TaxSector::Industrial => {
                self.budget_ui.industrial_tax = rate;
            }
        }

        let state = self.budget_tax_input_mut(sector);
        let text = rate.to_string();
        if *state != text {
            *state = text;
        }
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
            TaxSector::Residential => self.budget_ui.residential_tax,
            TaxSector::Commercial => self.budget_ui.commercial_tax,
            TaxSector::Industrial => self.budget_ui.industrial_tax,
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
            .as_str()
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .collect::<String>();

        if digits.len() > 3 {
            digits.truncate(3);
        }

        if digits.is_empty() {
            if *state != digits {
                *state = digits;
            }
            return;
        }

        let rate = digits.parse::<usize>().unwrap_or(0).min(100);
        let normalized = rate.to_string();
        if digits != normalized {
            digits = normalized;
        }

        if *state != digits {
            *state = digits;
        }
        self.set_budget_tax_rate(sector, rate, context);
    }

    pub fn restore_budget_tax_input_if_empty(&mut self, sector: TaxSector) {
        if self.budget_tax_input_mut(sector).is_empty() {
            let value = self.budget_slider_value(sector);
            self.set_budget_tax_ui_value(sector, value);
        }
    }

    pub fn handle_budget_action(&mut self, action: &Action, context: &AppContext) -> bool {
        if !self.is_budget_open() {
            return false;
        }

        match action {
            Action::MenuBack | Action::CharInput('b') | Action::CharInput('B') => {
                self.close_budget();
                true
            }
            Action::DeleteChar => {
                let sector = self.budget_ui.focused.tax_sector();
                self.budget_tax_input_mut(sector).pop();
                self.apply_budget_tax_input(sector, context);
                true
            }
            Action::CharInput(c) if c.is_ascii_digit() => {
                let sector = self.budget_ui.focused.tax_sector();
                let state = self.budget_tax_input_mut(sector);
                if state.len() < 3 {
                    state.push(*c);
                }
                self.apply_budget_tax_input(sector, context);
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
                self.desktop.update_drag(*col, *row);
                true
            }
            Action::MouseUp { .. } => {
                self.desktop.end_drag();
                true
            }
            _ => true,
        }
    }

    pub fn focus_budget_control_at(&mut self, col: u16, row: u16) {
        let Some(focus) = crate::ui::game::budget::focus_at_position(
            self.desktop.window(WindowId::Budget).x,
            self.desktop.window(WindowId::Budget).y,
            self.desktop.window(WindowId::Budget).width,
            self.desktop.window(WindowId::Budget).height,
            col,
            row,
        ) else {
            return;
        };
        self.budget_ui.focused = focus;
    }
}
