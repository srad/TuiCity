use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    symbols,
    text::Line,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

use crate::ui::view::StatisticsWindowViewModel;

pub fn render_statistics_content(frame: &mut Frame, area: Rect, view: &StatisticsWindowViewModel) {
    if area.width < 20 || area.height < 8 {
        return;
    }

    let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    render_summary(frame, sections[0], view);

    let rows = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);
    let top =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[0]);
    let bottom =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);

    render_series_chart(
        frame,
        top[0],
        &format!("Treasury  {}", fmt_money(view.current_treasury)),
        &series_from_i64(&view.treasury_history),
        crate::ui::theme::ui_palette().accent,
    );
    render_series_chart(
        frame,
        top[1],
        &format!("Population  {}", fmt_number(view.current_population)),
        &series_from_u64(&view.population_history),
        crate::ui::theme::ui_palette().success,
    );
    render_series_chart(
        frame,
        bottom[0],
        &format!("Annual Income  {}", fmt_money(view.current_income)),
        &series_from_i64(&view.income_history),
        crate::ui::theme::ui_palette().warning,
    );
    let power_balance = view.current_power_produced as i32 - view.current_power_consumed as i32;
    render_series_chart(
        frame,
        bottom[1],
        &format!("Power Balance  {} MW", power_balance),
        &series_from_i32(&view.power_balance_history),
        crate::ui::theme::ui_palette().info,
    );
}

fn render_summary(frame: &mut Frame, area: Rect, view: &StatisticsWindowViewModel) {
    let ui = crate::ui::theme::ui_palette();
    let lines = vec![
        Line::from(format!(
            "{}  |  Population {}  |  Treasury {}",
            view.city_name,
            fmt_number(view.current_population),
            fmt_money(view.current_treasury)
        )),
        Line::from(format!(
            "Annual Income {}  |  Power {} / {} MW  |  History {} months",
            fmt_money(view.current_income),
            view.current_power_consumed,
            view.current_power_produced,
            history_months(view)
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(
            Style::default()
                .fg(ui.text_primary)
                .bg(ui.popup_bg)
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}

fn render_series_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    points: &[(f64, f64)],
    color: ratatui::style::Color,
) {
    let ui = crate::ui::theme::ui_palette();
    if area.width < 10 || area.height < 5 {
        return;
    }

    if points.is_empty() {
        frame.render_widget(
            Paragraph::new("No history yet")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(ui.window_border))
                        .style(Style::default().bg(ui.popup_bg)),
                )
                .style(Style::default().fg(ui.text_muted).bg(ui.popup_bg)),
            area,
        );
        return;
    }

    let (min_y, max_y) = chart_bounds(points);
    let max_x = (points.len().saturating_sub(1)).max(1) as f64;
    let dataset = Dataset::default()
        .graph_type(GraphType::Line)
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(color))
        .data(points);

    frame.render_widget(
        Chart::new(vec![dataset])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(ui.window_border))
                    .style(Style::default().bg(ui.popup_bg)),
            )
            .style(Style::default().bg(ui.popup_bg))
            .x_axis(Axis::default().bounds([0.0, max_x]))
            .y_axis(Axis::default().bounds([min_y, max_y])),
        area,
    );
}

fn chart_bounds(points: &[(f64, f64)]) -> (f64, f64) {
    let mut min_y = points[0].1;
    let mut max_y = points[0].1;
    for &(_, value) in points.iter().skip(1) {
        min_y = min_y.min(value);
        max_y = max_y.max(value);
    }
    if (max_y - min_y).abs() < f64::EPSILON {
        let pad = min_y.abs().max(1.0);
        (min_y - pad, max_y + pad)
    } else {
        let pad = ((max_y - min_y) * 0.12).max(1.0);
        (min_y - pad, max_y + pad)
    }
}

fn history_months(view: &StatisticsWindowViewModel) -> usize {
    [
        view.treasury_history.len(),
        view.population_history.len(),
        view.income_history.len(),
        view.power_balance_history.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

fn series_from_i64(values: &[i64]) -> Vec<(f64, f64)> {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| (idx as f64, *value as f64))
        .collect()
}

fn series_from_u64(values: &[u64]) -> Vec<(f64, f64)> {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| (idx as f64, *value as f64))
        .collect()
}

fn series_from_i32(values: &[i32]) -> Vec<(f64, f64)> {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| (idx as f64, *value as f64))
        .collect()
}

fn fmt_number(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn fmt_money(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    if abs >= 1_000_000 {
        format!("{sign}${:.1}M", abs as f64 / 1_000_000.0)
    } else if abs >= 1_000 {
        format!("{sign}${:.1}k", abs as f64 / 1_000.0)
    } else {
        format!("{sign}${abs}")
    }
}
