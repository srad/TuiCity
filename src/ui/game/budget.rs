use crate::{
    app::screens::BudgetFocus,
    core::sim::{
        economy::{annual_tax_from_base, TaxRates},
        TaxSector,
    },
    ui::{theme, view::BudgetViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget},
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

fn sector_tax(base: u64, rates: TaxRates, sector: TaxSector) -> i64 {
    annual_tax_from_base(base, rates.get(sector))
}

fn budget_rows(area: Rect) -> [Rect; 3] {
    let rows = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(12),
        Constraint::Length(2),
    ])
    .split(area);
    [rows[0], rows[1], rows[2]]
}

fn budget_side(area: Rect) -> [Rect; 4] {
    let rows = budget_rows(area);
    let body = Layout::horizontal([Constraint::Percentage(54), Constraint::Percentage(46)]).split(rows[1]);
    let side = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Min(5),
    ])
    .split(body[1]);
    [side[0], side[1], side[2], side[3]]
}

pub fn focus_at_position(
    outer_x: u16,
    outer_y: u16,
    outer_width: u16,
    outer_height: u16,
    col: u16,
    row: u16,
) -> Option<BudgetFocus> {
    if outer_width < 3 || outer_height < 3 {
        return None;
    }
    let inner = Rect::new(
        outer_x.saturating_add(1),
        outer_y.saturating_add(1),
        outer_width.saturating_sub(2),
        outer_height.saturating_sub(2),
    );
    let side = budget_side(inner);
    if contains(side[0], col, row) {
        Some(BudgetFocus::ResidentialTax)
    } else if contains(side[1], col, row) {
        Some(BudgetFocus::CommercialTax)
    } else if contains(side[2], col, row) {
        Some(BudgetFocus::IndustrialTax)
    } else {
        None
    }
}

fn contains(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
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

fn render_tax_panel(
    buf: &mut Buffer,
    area: Rect,
    sector: TaxSector,
    value: usize,
    input: &str,
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
    let field_bg = if focused { theme::sector_color(sector) } else { ui.input_bg };
    let field_fg = if focused { ui.input_focus_fg } else { theme::sector_color(sector) };
    let visible_text = format!("{:>width$}", truncate(input, field_width as usize), width = field_width as usize);

    buf.set_string(
        inner.x,
        inner.y,
        truncate("Tax", field_x.saturating_sub(inner.x + 1) as usize),
        Style::default().fg(ui.text_secondary).bg(ui.window_bg),
    );
    buf.set_string(field_area.x, field_area.y, visible_text, Style::default().fg(field_fg).bg(field_bg));
    buf.set_string(
        percent_x,
        inner.y,
        "%",
        Style::default()
            .fg(theme::sector_color(sector))
            .bg(ui.window_bg)
            .add_modifier(Modifier::BOLD),
    );

    let bar = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    render_dos_bar(buf, bar, value, 100, theme::sector_color(sector), if focused { theme::sector_bg(sector) } else { ui.slider_bg }, ui);
}

fn render_dos_bar(
    buf: &mut Buffer,
    area: Rect,
    value: usize,
    max: usize,
    fill_fg: ratatui::style::Color,
    bg: ratatui::style::Color,
    ui: theme::UiPalette,
) {
    if area.width < 3 || area.height == 0 {
        return;
    }
    let inner_width = area.width.saturating_sub(2);
    let filled = ((value.min(max) as u32 * inner_width as u32) / max.max(1) as u32) as u16;
    buf.set_string(area.x, area.y, "[", Style::default().fg(ui.text_secondary).bg(bg));
    buf.set_string(area.x + area.width - 1, area.y, "]", Style::default().fg(ui.text_secondary).bg(bg));
    for i in 0..inner_width {
        let x = area.x + 1 + i;
        let (symbol, fg) = if i < filled { ("█", fill_fg) } else { ("░", ui.text_dim) };
        buf.set_string(x, area.y, symbol, Style::default().fg(fg).bg(bg));
    }
}

struct BudgetContent<'a> {
    view: &'a BudgetViewModel,
}

impl<'a> Widget for BudgetContent<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 30 || area.height < 16 {
            return;
        }

        let ui = theme::ui_palette();
        let tax_rates = self.view.tax_rates;
        let residential_tax = sector_tax(self.view.residential_population, tax_rates, TaxSector::Residential);
        let commercial_tax = sector_tax(self.view.commercial_jobs, tax_rates, TaxSector::Commercial);
        let industrial_tax = sector_tax(self.view.industrial_jobs, tax_rates, TaxSector::Industrial);
        let projected_total_tax = residential_tax + commercial_tax + industrial_tax;
        let projected_net = projected_total_tax - self.view.breakdown.total;
        let current_net = self.view.current_annual_tax - self.view.breakdown.total;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(' ');
                cell.set_bg(ui.budget_window_bg);
            }
        }

        let [cards_row, body_row, footer_row] = budget_rows(area);
        let cards = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(cards_row);

        render_card(
            buf,
            cards[0],
            "Treasury",
            &fmt_money(self.view.treasury),
            Style::default().fg(ui.text_muted),
            Style::default().fg(if self.view.treasury >= 0 { ui.success } else { ui.danger }),
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
            &fmt_abs(self.view.residential_population),
            Style::default().fg(ui.text_muted),
            Style::default().fg(ui.sector_residential),
            ui,
        );
        render_card(
            buf,
            cards[3],
            "Jobs",
            &format!("C {} / I {}", fmt_abs(self.view.commercial_jobs), fmt_abs(self.view.industrial_jobs)),
            Style::default().fg(ui.text_muted),
            Style::default().fg(ui.text_primary),
            ui,
        );

        let body = Layout::horizontal([Constraint::Percentage(54), Constraint::Percentage(46)]).split(body_row);

        let ops_block = Block::default()
            .borders(Borders::ALL)
            .title(" Operations ")
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.budget_window_bg));
        let ops_inner = ops_block.inner(body[0]);
        ops_block.render(body[0], buf);

        let mut row = ops_inner.y;
        let summary_rows = [
            ("Residential tax", residential_tax, ui.sector_residential),
            ("Commercial tax", commercial_tax, ui.sector_commercial),
            ("Industrial tax", industrial_tax, ui.sector_industrial),
            ("Maintenance", -self.view.breakdown.total, ui.danger),
            ("Net", projected_net, if projected_net >= 0 { ui.success } else { ui.danger }),
        ];
        for (label, value, color) in summary_rows {
            if row >= ops_inner.y + ops_inner.height {
                break;
            }
            let value_text = if value >= 0 { format!("+{}", fmt_money(value)) } else { fmt_money(value) };
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
        for (label, value) in [
            ("Roads", self.view.breakdown.roads),
            ("Power Lines", self.view.breakdown.power_lines),
            ("Power Plants", self.view.breakdown.power_plants),
            ("Police", self.view.breakdown.police),
            ("Fire Dept", self.view.breakdown.fire),
            ("Parks", self.view.breakdown.parks),
        ] {
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

        let [residential_area, commercial_area, industrial_area, forecast_area] = budget_side(area);
        render_tax_panel(
            buf,
            residential_area,
            TaxSector::Residential,
            self.view.tax_rates.residential as usize,
            &self.view.residential_input,
            self.view.focused == BudgetFocus::ResidentialTax,
            ui,
        );
        render_tax_panel(
            buf,
            commercial_area,
            TaxSector::Commercial,
            self.view.tax_rates.commercial as usize,
            &self.view.commercial_input,
            self.view.focused == BudgetFocus::CommercialTax,
            ui,
        );
        render_tax_panel(
            buf,
            industrial_area,
            TaxSector::Industrial,
            self.view.tax_rates.industrial as usize,
            &self.view.industrial_input,
            self.view.focused == BudgetFocus::IndustrialTax,
            ui,
        );

        let forecast_block = Block::default()
            .borders(Borders::ALL)
            .title(" Forecast ")
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.window_bg));
        let forecast_inner = forecast_block.inner(forecast_area);
        forecast_block.render(forecast_area, buf);
        let mut row = forecast_inner.y;
        for (label, value, color) in [
            ("Residential", residential_tax, ui.sector_residential),
            ("Commercial", commercial_tax, ui.sector_commercial),
            ("Industrial", industrial_tax, ui.sector_industrial),
            ("Total tax", projected_total_tax, ui.success),
            ("Net", projected_net, if projected_net >= 0 { ui.success } else { ui.danger }),
        ] {
            if row >= forecast_inner.y + forecast_inner.height {
                break;
            }
            let value_text = if value >= 0 { format!("+{}", fmt_money(value)) } else { fmt_money(value) };
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
            footer_row.x,
            footer_row.y,
            truncate("Up/Down focus  Left/Right +/-1  Type 0-100", footer_row.width as usize),
            Style::default().fg(ui.text_dim).bg(ui.budget_window_bg),
        );
        if footer_row.height > 1 {
            buf.set_string(
                footer_row.x,
                footer_row.y + 1,
                truncate("Click sector panel to focus  [X], Esc, or B closes", footer_row.width as usize),
                Style::default().fg(ui.text_dim).bg(ui.budget_window_bg),
            );
        }
    }
}

pub fn render_budget_content(buf: &mut Buffer, inner: Rect, view: &BudgetViewModel) {
    BudgetContent { view }.render(inner, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::{map::Map, sim::SimState}, ui::view::BudgetViewModel};
    use ratatui::{buffer::Buffer, layout::Rect};

    fn render_budget_lines(width: u16, height: u16) -> Vec<String> {
        let _map = Map::new(32, 32);
        let sim = SimState::default();
        let view = BudgetViewModel::from_sim(
            &sim,
            BudgetFocus::ResidentialTax,
            sim.tax_rates,
            "9".to_string(),
            "9".to_string(),
            "9".to_string(),
        );
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_budget_content(&mut buf, area, &view);

        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect()
    }

    fn right_column_rows(width: u16, height: u16) -> [u16; 4] {
        let area = Rect::new(0, 0, width, height);
        let [_, body, _] = budget_rows(area);
        let body = Layout::horizontal([Constraint::Percentage(54), Constraint::Percentage(46)]).split(body);
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

        assert!(lines[residential_row as usize].contains("Residential"));
        assert!(lines[commercial_row as usize].contains("Commercial"));
        assert!(lines[industrial_row as usize].contains("Industrial"));
        assert!(lines[forecast_row as usize].contains("Forecast"));
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
