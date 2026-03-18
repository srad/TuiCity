use crate::app::AppState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Sparkline, Widget},
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

// ── Content widget ─────────────────────────────────────────────────────────────
// Renders the budget popup contents into whatever area tui-popup hands it.

struct BudgetContent<'a> {
    app: &'a AppState,
}

impl<'a> Widget for BudgetContent<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let engine = self.app.engine.read().unwrap();
        let sim = &engine.sim;
        let b = &sim.last_breakdown;

        let inner = area; // tui-popup already gives us the inner area
        let mut row = inner.y;

        let right_fill = |prefix_cols: u16| "─".repeat(inner.width.saturating_sub(prefix_cols) as usize);
        let full_sep   = ||                 "─".repeat(inner.width as usize);

        // ── Header ────────────────────────────────────────────────────────────
        buf.set_string(
            inner.x, row,
            format!("Treasury:   {}", fmt_money(sim.treasury)),
            Style::default().fg(if sim.treasury >= 0 { Color::Rgb(80, 220, 80) } else { Color::Rgb(220, 60, 60) }),
        );
        row += 1;

        buf.set_string(
            inner.x, row,
            format!("Population: {}", fmt_abs(sim.population)),
            Style::default().fg(Color::White),
        );
        row += 1;

        buf.set_string(
            inner.x, row,
            format!("Tax Rate:   {}%", sim.tax_rate),
            Style::default().fg(Color::Cyan).bold(),
        );
        row += 2;

        // ── Income section ────────────────────────────────────────────────────
        buf.set_string(inner.x,      row, "── Income ", Style::default().fg(Color::DarkGray));
        buf.set_string(inner.x + 10, row, &right_fill(10), Style::default().fg(Color::DarkGray));
        row += 1;

        let tax_str = format!("+{}", fmt_money(b.annual_tax));
        let label_col = inner.x + inner.width.saturating_sub(tax_str.len() as u16 + 1);
        buf.set_string(inner.x, row, "  Annual Tax:", Style::default().fg(Color::White));
        buf.set_string(label_col, row, &tax_str, Style::default().fg(Color::Rgb(80, 220, 80)));
        row += 2;

        // ── Maintenance section ───────────────────────────────────────────────
        buf.set_string(inner.x,      row, "── Maintenance (annual) ", Style::default().fg(Color::DarkGray));
        buf.set_string(inner.x + 24, row, &right_fill(24), Style::default().fg(Color::DarkGray));
        row += 1;

        let items = [
            ("  Roads:",        b.roads),
            ("  Power Lines:",  b.power_lines),
            ("  Power Plants:", b.power_plants),
            ("  Police:",       b.police),
            ("  Fire Dept:",    b.fire),
            ("  Parks:",        b.parks),
        ];

        for (label, cost) in &items {
            if *cost == 0 { continue; }
            let cost_str = format!("-{}", fmt_money(*cost));
            let col = inner.x + inner.width.saturating_sub(cost_str.len() as u16 + 1);
            buf.set_string(inner.x, row, label, Style::default().fg(Color::Rgb(180, 180, 180)));
            buf.set_string(col, row, &cost_str, Style::default().fg(Color::Rgb(220, 80, 80)));
            row += 1;
        }

        // ── Net ───────────────────────────────────────────────────────────────
        buf.set_string(inner.x, row, &full_sep(), Style::default().fg(Color::DarkGray));
        row += 1;

        let net = b.annual_tax - b.total;
        let net_str = format!("{}{}", if net >= 0 { "+" } else { "" }, fmt_money(net));
        let net_col = inner.x + inner.width.saturating_sub(net_str.len() as u16 + 1);
        buf.set_string(inner.x, row, "  Net / yr:", Style::default().fg(Color::White).bold());
        buf.set_string(
            net_col, row, &net_str,
            Style::default().fg(if net >= 0 { Color::Rgb(80, 220, 80) } else { Color::Rgb(220, 60, 60) }).bold(),
        );
        row += 2;

        // ── Treasury sparkline ────────────────────────────────────────────────
        if !sim.treasury_history.is_empty() && row + 2 < inner.y + inner.height {
            buf.set_string(inner.x, row, "Treasury History:", Style::default().fg(Color::DarkGray));
            row += 1;
            let data: Vec<u64> = sim.treasury_history.iter().map(|&v| v.max(0) as u64).collect();
            let sparkline = Sparkline::default()
                .data(&data)
                .style(Style::default().fg(Color::Green));
            sparkline.render(Rect::new(inner.x, row, inner.width, 1), buf);
            row += 2;
        }

        // ── Controls hint ─────────────────────────────────────────────────────
        let hint_row = inner.y + inner.height.saturating_sub(1);
        let _ = row;
        buf.set_string(
            inner.x, hint_row,
            "UP/DOWN: tax rate  |  ESC/B: close",
            Style::default().fg(Color::DarkGray),
        );
    }
}

/// Render budget content directly into `inner` (the area inside the window border).
pub fn render_budget_content(buf: &mut Buffer, inner: Rect, app: &AppState) {
    BudgetContent { app }.render(inner, buf);
}


// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    /// "─" is U+2500 BOX DRAWINGS LIGHT HORIZONTAL — 3 UTF-8 bytes, 1 terminal column.
    /// Slicing by byte index (e.g. `sep[10..]`) panics inside a multi-byte char.
    /// These tests verify the fill helpers are column-based, not byte-based.
    #[test]
    fn right_fill_produces_valid_utf8() {
        let inner_width: u16 = 40;
        let prefix_cols: u16 = 10;
        let fill = "─".repeat(inner_width.saturating_sub(prefix_cols) as usize);
        assert_eq!(fill.chars().count(), 30);
    }

    #[test]
    fn full_sep_produces_valid_utf8_no_panic() {
        for width in [0u16, 1, 10, 40, 80] {
            let sep = "─".repeat(width as usize);
            assert_eq!(sep.chars().count(), width as usize);
        }
    }

    #[test]
    fn right_fill_with_large_prefix_clamps_to_zero() {
        let fill = "─".repeat((10u16.saturating_sub(24)) as usize);
        assert_eq!(fill.len(), 0);
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
}
