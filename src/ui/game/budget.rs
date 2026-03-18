use crate::{
    app::AppState,
    app::screens::{BudgetFocus, BudgetUiState},
    core::sim::{
        TaxSector,
        economy::{annual_tax_from_base, TaxRates},
    },
    ui::theme,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

fn fmt_money(n: i64) -> String {
    if n < 0 {
        return format!("-${}", fmt_abs((-n) as u64));
    }
    format!("${}", fmt_abs(n as u64))
}

fn fmt_abs(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn truncate(s: impl AsRef<str>, max: usize) -> String {
    let s = s.as_ref();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

fn configure_slider_state(
    area: Rect,
    state: &mut rat_widget::slider::SliderState<usize>,
) {
    state.area = area;
    state.inner = area;
    state.lower_bound = Rect::default();
    state.upper_bound = Rect::default();
    state.track = area;
    state.scale_len = area.width.saturating_sub(1);
    state.direction = Direction::Horizontal;

    let range = state.range;
    let span = range.1.saturating_sub(range.0).max(1);
    let relative = state.value.saturating_sub(range.0).min(span);
    let filled = ((relative as u32 * area.width as u32) / span as u32) as u16;
    let knob_x = area.x + filled.saturating_sub(1).min(area.width.saturating_sub(1));
    state.knob = Rect::new(knob_x, area.y, 1.min(area.width), area.height);
}

fn sector_slider(
    area: Rect,
    buf: &mut Buffer,
    state: &mut rat_widget::slider::SliderState<usize>,
    focused: bool,
    sector: TaxSector,
    ui: theme::UiPalette,
) {
    if area.height < 1 || area.width < 12 {
        return;
    }
    configure_slider_state(area, state);

    let track_bg = if focused { theme::sector_bg(sector) } else { ui.slider_bg };
    let fill_fg = theme::sector_color(sector);
    let range = state.range;
    let span = range.1.saturating_sub(range.0).max(1);
    let relative = state.value.saturating_sub(range.0).min(span);
    let inner_width = area.width.saturating_sub(2);
    let filled = if inner_width == 0 {
        0
    } else {
        ((relative as u32 * inner_width as u32) / span as u32) as u16
    };

    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_symbol("[");
        cell.set_fg(ui.text_secondary);
        cell.set_bg(track_bg);
    }
    if area.width > 1 {
        if let Some(cell) = buf.cell_mut((area.x + area.width - 1, area.y)) {
            cell.set_symbol("]");
            cell.set_fg(ui.text_secondary);
            cell.set_bg(track_bg);
        }
    }

    for i in 0..inner_width {
        let x = area.x + 1 + i;
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            if i < filled {
                cell.set_symbol("█");
                cell.set_fg(fill_fg);
                cell.set_bg(track_bg);
            } else {
                cell.set_symbol("░");
                cell.set_fg(ui.text_dim);
                cell.set_bg(track_bg);
            }
        }
    }
}

fn render_tax_panel(
    buf: &mut Buffer,
    area: Rect,
    sector: TaxSector,
    state: &mut rat_widget::slider::SliderState<usize>,
    input_state: &mut rat_widget::text_input::TextInputState,
    focused: bool,
    ui: theme::UiPalette,
) {
    if area.width < 16 || area.height < 4 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", sector.label()))
        .title_style(
            Style::default()
                .fg(theme::sector_color(sector))
                .add_modifier(if focused { Modifier::BOLD } else { Modifier::empty() }),
        )
        .border_style(Style::default().fg(if focused { theme::sector_color(sector) } else { ui.window_border }))
        .style(Style::default().bg(ui.window_bg));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height < 2 {
        return;
    }

    let field_width = 3.min(inner.width);
    let percent_x = inner.x + inner.width.saturating_sub(1);
    let field_x = percent_x.saturating_sub(field_width);
    let field_area = Rect::new(field_x, inner.y, field_width, 1);
    buf.set_string(
        inner.x,
        inner.y,
        truncate("Tax", field_x.saturating_sub(inner.x + 1) as usize),
        Style::default().fg(ui.text_secondary).bg(ui.window_bg),
    );

    let mut text_input = rat_widget::text_input::TextInput::new()
        .style(Style::default().fg(theme::sector_color(sector)).bg(ui.input_bg))
        .focus_style(Style::default().fg(ui.input_focus_fg).bg(theme::sector_color(sector)))
        .invalid_style(Style::default().fg(ui.danger));
    if focused {
        text_input = text_input.select_style(Style::default().bg(theme::sector_bg(sector)));
    }
    StatefulWidget::render(text_input, field_area, buf, input_state);

    let field_bg = if focused {
        theme::sector_color(sector)
    } else {
        ui.input_bg
    };
    let field_fg = if focused {
        ui.input_focus_fg
    } else {
        theme::sector_color(sector)
    };
    let visible_text = format!("{:>width$}", input_state.text(), width = field_width as usize);
    buf.set_string(
        field_area.x,
        field_area.y,
        visible_text,
        Style::default().fg(field_fg).bg(field_bg),
    );
    buf.set_string(
        percent_x,
        inner.y,
        "%",
        Style::default()
            .fg(theme::sector_color(sector))
            .bg(ui.window_bg)
            .add_modifier(Modifier::BOLD),
    );
    sector_slider(Rect::new(inner.x, inner.y + 1, inner.width, 1), buf, state, focused, sector, ui);
}

fn render_card(
    buf: &mut Buffer,
    area: Rect,
    title: &str,
    value: &str,
    title_style: Style,
    value_style: Style,
    ui: theme::UiPalette,
) {
    if area.width < 4 || area.height < 3 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui.window_border))
        .style(Style::default().bg(ui.window_bg));
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height > 0 {
        buf.set_string(inner.x, inner.y, truncate(title, inner.width as usize), title_style.bg(ui.window_bg));
    }
    if inner.height > 1 {
        buf.set_string(
            inner.x,
            inner.y + 1,
            truncate(value, inner.width as usize),
            value_style.bg(ui.window_bg).add_modifier(Modifier::BOLD),
        );
    }
}

fn sector_tax(base: u64, rates: TaxRates, sector: TaxSector) -> i64 {
    annual_tax_from_base(base, rates.get(sector))
}

struct BudgetContent<'a> {
    app: &'a AppState,
    state: &'a mut BudgetUiState,
}

impl<'a> Widget for BudgetContent<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 30 || area.height < 16 {
            return;
        }

        let ui = theme::ui_palette();
        let engine = self.app.engine.read().unwrap();
        let sim = &engine.sim;
        let breakdown = &sim.last_breakdown;
        let tax_rates = TaxRates {
            residential: self.state.residential_tax.value() as u8,
            commercial: self.state.commercial_tax.value() as u8,
            industrial: self.state.industrial_tax.value() as u8,
        };

        let residential_tax = sector_tax(sim.residential_population, tax_rates, TaxSector::Residential);
        let commercial_tax = sector_tax(sim.commercial_jobs, tax_rates, TaxSector::Commercial);
        let industrial_tax = sector_tax(sim.industrial_jobs, tax_rates, TaxSector::Industrial);
        let projected_total_tax = residential_tax + commercial_tax + industrial_tax;
        let projected_net = projected_total_tax - breakdown.total;
        let current_net = breakdown.annual_tax - breakdown.total;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(' ');
                cell.set_bg(ui.budget_window_bg);
            }
        }

        let rows = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(area);

        let cards = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(rows[0]);

        render_card(
            buf,
            cards[0],
            "Treasury",
            &fmt_money(sim.treasury),
            Style::default().fg(ui.text_muted),
            Style::default().fg(if sim.treasury >= 0 { ui.success } else { ui.danger }),
            ui,
        );
        render_card(
            buf,
            cards[1],
            "Annual Net",
            &format!("{}{}/yr", if current_net >= 0 { "+" } else { "" }, fmt_money(current_net)),
            Style::default().fg(ui.text_muted),
            Style::default().fg(if current_net >= 0 { ui.success } else { ui.danger }),
            ui,
        );
        render_card(
            buf,
            cards[2],
            "Residents",
            &fmt_abs(sim.residential_population),
            Style::default().fg(ui.text_muted),
            Style::default().fg(ui.sector_residential),
            ui,
        );
        render_card(
            buf,
            cards[3],
            "Jobs",
            &format!("C {} / I {}", fmt_abs(sim.commercial_jobs), fmt_abs(sim.industrial_jobs)),
            Style::default().fg(ui.text_muted),
            Style::default().fg(ui.text_primary),
            ui,
        );

        let body = Layout::horizontal([Constraint::Percentage(54), Constraint::Percentage(46)]).split(rows[1]);

        let ops_block = Block::default()
            .borders(Borders::ALL)
            .title(" Operations ")
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.budget_window_bg));
        let ops_inner = ops_block.inner(body[0]);
        ops_block.render(body[0], buf);

        let expense_rows = [
            ("Roads", breakdown.roads),
            ("Power Lines", breakdown.power_lines),
            ("Power Plants", breakdown.power_plants),
            ("Police", breakdown.police),
            ("Fire Dept", breakdown.fire),
            ("Parks", breakdown.parks),
        ];

        let mut row = ops_inner.y;
        let summary_rows = [
            ("Residential tax", residential_tax, ui.sector_residential),
            ("Commercial tax", commercial_tax, ui.sector_commercial),
            ("Industrial tax", industrial_tax, ui.sector_industrial),
            ("Maintenance", -breakdown.total, ui.danger),
            ("Net", projected_net, if projected_net >= 0 { ui.success } else { ui.danger }),
        ];
        for (label, value, color) in summary_rows {
            if row >= ops_inner.y + ops_inner.height {
                break;
            }
            let value_text = if value >= 0 {
                format!("+{}", fmt_money(value))
            } else {
                fmt_money(value)
            };
            let value_width = value_text.chars().count() as u16;
            let value_x = ops_inner.x + ops_inner.width.saturating_sub(value_width);
            buf.set_string(
                ops_inner.x,
                row,
                truncate(label, ops_inner.width.saturating_sub(value_width + 1) as usize),
                Style::default().fg(ui.text_secondary).bg(ui.budget_window_bg),
            );
            buf.set_string(value_x, row, &value_text, Style::default().fg(color).bg(ui.budget_window_bg));
            row += 1;
        }

        row += 1;
        for (label, value) in expense_rows {
            if row >= ops_inner.y + ops_inner.height {
                break;
            }
            let value_text = format!("-{}", fmt_money(value));
            let value_width = value_text.chars().count() as u16;
            let value_x = ops_inner.x + ops_inner.width.saturating_sub(value_width);
            buf.set_string(
                ops_inner.x,
                row,
                truncate(label, ops_inner.width.saturating_sub(value_width + 1) as usize),
                Style::default().fg(ui.text_muted).bg(ui.budget_window_bg),
            );
            buf.set_string(value_x, row, &value_text, Style::default().fg(ui.danger).bg(ui.budget_window_bg));
            row += 1;
        }

        let side = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(5),
        ])
        .split(body[1]);
        render_tax_panel(
            buf,
            side[0],
            TaxSector::Residential,
            &mut self.state.residential_tax,
            &mut self.state.residential_tax_input,
            self.state.focused == BudgetFocus::ResidentialTax,
            ui,
        );
        render_tax_panel(
            buf,
            side[1],
            TaxSector::Commercial,
            &mut self.state.commercial_tax,
            &mut self.state.commercial_tax_input,
            self.state.focused == BudgetFocus::CommercialTax,
            ui,
        );
        render_tax_panel(
            buf,
            side[2],
            TaxSector::Industrial,
            &mut self.state.industrial_tax,
            &mut self.state.industrial_tax_input,
            self.state.focused == BudgetFocus::IndustrialTax,
            ui,
        );

        let forecast_block = Block::default()
            .borders(Borders::ALL)
            .title(" Forecast ")
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.window_bg));
        let forecast_inner = forecast_block.inner(side[3]);
        forecast_block.render(side[3], buf);
        let forecast_rows = [
            ("Residential", residential_tax, ui.sector_residential),
            ("Commercial", commercial_tax, ui.sector_commercial),
            ("Industrial", industrial_tax, ui.sector_industrial),
            ("Total tax", projected_total_tax, ui.success),
            ("Net", projected_net, if projected_net >= 0 { ui.success } else { ui.danger }),
        ];
        let mut row = forecast_inner.y;
        for (label, value, color) in forecast_rows {
            if row >= forecast_inner.y + forecast_inner.height {
                break;
            }
            let value_text = if value >= 0 {
                format!("+{}", fmt_money(value))
            } else {
                fmt_money(value)
            };
            let value_width = value_text.chars().count() as u16;
            let value_x = forecast_inner.x + forecast_inner.width.saturating_sub(value_width);
            buf.set_string(
                forecast_inner.x,
                row,
                truncate(label, forecast_inner.width.saturating_sub(value_width + 1) as usize),
                Style::default().fg(ui.text_secondary).bg(ui.window_bg),
            );
            buf.set_string(value_x, row, &value_text, Style::default().fg(color).bg(ui.window_bg));
            row += 1;
        }

        buf.set_string(
            rows[2].x,
            rows[2].y,
            truncate("Up/Down focus  Type 0-100 to set sector tax", rows[2].width as usize),
            Style::default().fg(ui.text_dim).bg(ui.budget_window_bg),
        );
        if rows[2].height > 1 {
            buf.set_string(
                rows[2].x,
                rows[2].y + 1,
                truncate("Mouse: click a field to edit  [X], Esc, or B closes", rows[2].width as usize),
                Style::default().fg(ui.text_dim).bg(ui.budget_window_bg),
            );
        }
    }
}

pub fn render_budget_content(
    buf: &mut Buffer,
    inner: Rect,
    app: &AppState,
    state: &mut BudgetUiState,
) {
    BudgetContent { app, state }.render(inner, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::AppState,
        app::screens::BudgetUiState,
        core::{engine::SimulationEngine, map::Map, sim::SimState},
    };
    use ratatui::{buffer::Buffer, layout::Rect};
    use std::sync::{Arc, RwLock};

    fn render_budget_lines(width: u16, height: u16) -> Vec<String> {
        let app = AppState {
            screens: Vec::new(),
            engine: Arc::new(RwLock::new(SimulationEngine::new(Map::new(32, 32), SimState::default()))),
            cmd_tx: None,
            running: true,
        };
        let mut state = BudgetUiState::new();
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_budget_content(&mut buf, area, &app, &mut state);

        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    fn right_column_rows(width: u16, height: u16) -> [u16; 4] {
        let area = Rect::new(0, 0, width, height);
        let rows = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(area);
        let body = Layout::horizontal([Constraint::Percentage(54), Constraint::Percentage(46)]).split(rows[1]);
        let side = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(5),
        ])
        .split(body[1]);
        [side[0].y, side[1].y, side[2].y, side[3].y]
    }

    fn find_row(lines: &[String], needle: &str) -> usize {
        lines.iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("missing line containing {needle:?}"))
    }

    #[test]
    fn fmt_money_positive() {
        assert_eq!(super::fmt_money(1234), "$1,234");
    }

    #[test]
    fn fmt_money_negative() {
        assert_eq!(super::fmt_money(-500), "-$500");
    }

    #[test]
    fn fmt_money_zero() {
        assert_eq!(super::fmt_money(0), "$0");
    }

    #[test]
    fn sector_tax_uses_current_formula() {
        let rates = TaxRates::default();
        assert_eq!(super::sector_tax(1_000, rates, TaxSector::Residential), 5_000);
    }

    #[test]
    fn budget_render_shows_all_sector_panels_and_forecast() {
        let lines = render_budget_lines(72, 24);
        let [residential_row, commercial_row, industrial_row, forecast_row] = right_column_rows(72, 24);

        assert!(lines[residential_row as usize].contains("Residential"), "res row {residential_row}: {:?}", lines[residential_row as usize]);
        assert!(lines[commercial_row as usize].contains("Commercial"), "comm row {commercial_row}: {:?}", lines[commercial_row as usize]);
        assert!(lines[industrial_row as usize].contains("Industrial"), "ind row {industrial_row}: {:?}", lines[industrial_row as usize]);
        assert!(lines[forecast_row as usize].contains("Forecast"), "forecast row {forecast_row}: {:?}", lines[forecast_row as usize]);
    }

    #[test]
    fn budget_render_keeps_forecast_below_sector_panels() {
        let lines = render_budget_lines(72, 24);
        let [residential, commercial, industrial, expected_forecast] = right_column_rows(72, 24);
        let forecast = find_row(&lines, "Forecast") as u16;

        assert!(residential < commercial);
        assert!(commercial < industrial);
        assert!(industrial < forecast);
        assert_eq!(forecast, expected_forecast);
    }
}
