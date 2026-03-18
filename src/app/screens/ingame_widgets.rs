use crate::{app::Tool, core::sim::TaxSector};

use super::{AppContext, BudgetFocus, InGameScreen};

impl InGameScreen {
    pub fn select_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        self.line_drag = None;
        self.rect_drag = None;
        self.show_plant_info = false;
    }

    pub fn handle_popup_close_event(&mut self, event: &crossterm::event::Event) -> bool {
        use rat_widget::event::ButtonOutcome;

        if self.show_plant_info {
            let out_close = rat_widget::button::handle_mouse_events(&mut self.plant_close_btn, event);
            if out_close == ButtonOutcome::Pressed {
                self.show_plant_info = false;
                self.popup_input_consumed = true;
                return true;
            }
        }

        if self.is_budget_open {
            let out_close = rat_widget::button::handle_mouse_events(&mut self.budget_close_btn, event);
            if out_close == ButtonOutcome::Pressed {
                self.close_budget();
                self.popup_input_consumed = true;
                return true;
            }
        }

        if self.inspect_pos.is_some() {
            let out_close = rat_widget::button::handle_mouse_events(&mut self.inspect_close_btn, event);
            if out_close == ButtonOutcome::Pressed {
                self.inspect_pos = None;
                self.popup_input_consumed = true;
                return true;
            }
        }

        false
    }

    pub fn handle_budget_widget_event(&mut self, event: &crossterm::event::Event, context: &AppContext) -> bool {
        use rat_widget::event::{SliderOutcome, TextOutcome};

        if !self.is_budget_open {
            return false;
        }

        let allow_budget_arrow_adjust = matches!(
            event,
            crossterm::event::Event::Key(crossterm::event::KeyEvent {
                code: crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Right,
                kind,
                ..
            }) if *kind != crossterm::event::KeyEventKind::Release
        );
        let input_outcomes = [
            (
                if allow_budget_arrow_adjust {
                    TextOutcome::Continue
                } else {
                    rat_widget::text_input::handle_events(
                        &mut self.budget_ui.residential_tax_input,
                        self.budget_ui.focused == BudgetFocus::ResidentialTax,
                        event,
                    )
                },
                BudgetFocus::ResidentialTax,
                TaxSector::Residential,
            ),
            (
                if allow_budget_arrow_adjust {
                    TextOutcome::Continue
                } else {
                    rat_widget::text_input::handle_events(
                        &mut self.budget_ui.commercial_tax_input,
                        self.budget_ui.focused == BudgetFocus::CommercialTax,
                        event,
                    )
                },
                BudgetFocus::CommercialTax,
                TaxSector::Commercial,
            ),
            (
                if allow_budget_arrow_adjust {
                    TextOutcome::Continue
                } else {
                    rat_widget::text_input::handle_events(
                        &mut self.budget_ui.industrial_tax_input,
                        self.budget_ui.focused == BudgetFocus::IndustrialTax,
                        event,
                    )
                },
                BudgetFocus::IndustrialTax,
                TaxSector::Industrial,
            ),
        ];

        for (outcome, focus, sector) in input_outcomes {
            if outcome != TextOutcome::Continue {
                self.budget_input_consumed = true;
                self.budget_ui.focused = focus;
                if outcome == TextOutcome::TextChanged {
                    self.apply_budget_tax_input(sector, context);
                }
                return true;
            }
        }

        let slider_outcomes = [
            (
                rat_widget::slider::handle_mouse_events(&mut self.budget_ui.residential_tax, event),
                BudgetFocus::ResidentialTax,
                TaxSector::Residential,
            ),
            (
                rat_widget::slider::handle_mouse_events(&mut self.budget_ui.commercial_tax, event),
                BudgetFocus::CommercialTax,
                TaxSector::Commercial,
            ),
            (
                rat_widget::slider::handle_mouse_events(&mut self.budget_ui.industrial_tax, event),
                BudgetFocus::IndustrialTax,
                TaxSector::Industrial,
            ),
        ];

        for (outcome, focus, sector) in slider_outcomes {
            if outcome != SliderOutcome::Continue {
                self.budget_input_consumed = true;
                self.budget_ui.focused = focus;
                if matches!(outcome, SliderOutcome::Changed | SliderOutcome::Value) {
                    let value = self.budget_slider_value(sector);
                    self.set_budget_tax_rate(sector, value, context);
                }
                return true;
            }
        }

        false
    }

    pub fn handle_power_popup_event(&mut self, event: &crossterm::event::Event) -> bool {
        use rat_widget::event::ButtonOutcome;

        if !self.show_plant_info {
            return false;
        }

        let out_coal = rat_widget::button::handle_events(&mut self.coal_picker_btn, true, event);
        if out_coal == ButtonOutcome::Pressed {
            self.select_tool(Tool::PowerPlantCoal);
            return true;
        }
        let out_gas = rat_widget::button::handle_events(&mut self.gas_picker_btn, true, event);
        if out_gas == ButtonOutcome::Pressed {
            self.select_tool(Tool::PowerPlantGas);
            return true;
        }

        false
    }

    pub fn handle_toolbar_event(&mut self, event: &crossterm::event::Event) -> bool {
        use rat_widget::event::ButtonOutcome;

        let mut pressed_tool = None;
        for (tool, state) in self.toolbar_btn_states.iter_mut() {
            let outcome = rat_widget::button::handle_events(state, true, event);
            if outcome == ButtonOutcome::Pressed {
                pressed_tool = Some(*tool);
                break;
            }
        }

        if let Some(tool) = pressed_tool {
            if tool == Tool::PowerPlantPicker {
                self.show_plant_info = !self.show_plant_info;
            } else {
                self.select_tool(tool);
            }
            return true;
        }

        false
    }
}
