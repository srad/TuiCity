use super::common::{self, SynthwaveBackground};
use crate::{
    app::{screens::ThemeSettingsState, ClickArea},
    ui::{theme, view::ThemeSettingsViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

pub fn render_theme_settings(
    frame: &mut Frame,
    area: Rect,
    view: &ThemeSettingsViewModel,
    state: &mut ThemeSettingsState,
) {
    state.row_areas.clear();
    common::paint_synthwave(
        frame.buffer_mut(),
        area,
        SynthwaveBackground {
            lit_windows: true,
            ..Default::default()
        },
    );

    render_title(frame.buffer_mut(), area, area.y + 1);

    let layout = common::centered_panel(area, 54, 72, 28, 3);
    common::render_bordered_panel(frame, &layout, "PALETTE LAB");
    render_panel_content(frame, &layout, view, state);

    common::render_footer(
        frame.buffer_mut(),
        area,
        "Arrow Keys Preview  •  Enter Back  •  Esc Back  •  Shift+P Cycle",
    );
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16) {
    common::set_centered_string(
        buf,
        area.x,
        y,
        area.width,
        "Theme Settings",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    );
    common::set_centered_string(
        buf,
        area.x,
        y + 1,
        area.width,
        "preview palettes live",
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Reset),
    );
}

fn render_panel_content(
    frame: &mut Frame,
    layout: &common::PanelLayout,
    view: &ThemeSettingsViewModel,
    state: &mut ThemeSettingsState,
) {
    let inner = layout.inner;
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    let hint = format!("Current: {}", view.active.label());
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            common::truncate(&hint, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55))
            .add_modifier(Modifier::BOLD),
    );

    for (idx, preset) in view.themes.iter().copied().enumerate() {
        let row_y = inner.y + 2 + idx as u16 * 3;
        if row_y + 1 >= inner.y + inner.height {
            break;
        }
        state.row_areas.push(ClickArea {
            x: inner.x,
            y: row_y,
            width: inner.width,
            height: 2,
        });

        let selected = idx == view.selected;
        let palette = theme::palette_for(preset);
        let row_style = if selected {
            Style::default()
                .fg(Color::Rgb(28, 28, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Rgb(238, 232, 225))
                .bg(Color::Rgb(56, 42, 78))
        };

        let blank = format!("{:^width$}", " ", width = inner.width as usize);
        buf.set_string(inner.x, row_y, blank.clone(), row_style);
        buf.set_string(inner.x, row_y + 1, blank, row_style);

        let name = if preset == view.active {
            format!("{}  (ACTIVE)", preset.label())
        } else {
            preset.label().to_string()
        };
        buf.set_string(
            inner.x + 2,
            row_y,
            common::truncate(&name, inner.width.saturating_sub(4) as usize),
            row_style,
        );

        render_swatch(
            buf,
            inner.x + 2,
            row_y + 1,
            palette.title,
            palette.selection_bg,
            palette.accent,
            palette.info,
            palette.success,
            palette.warning,
            row_style.bg.unwrap_or(Color::Rgb(56, 42, 78)),
        );
    }

    // Back button after themes
    let back_idx = view.themes.len();
    let back_y = inner.y + 2 + back_idx as u16 * 3;
    if back_y + 2 < inner.y + inner.height {
        let area = common::render_back_button(buf, inner, back_y, view.selected == back_idx);
        state.row_areas.push(area);
    }
}

fn render_swatch(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    a: Color,
    b: Color,
    c: Color,
    d: Color,
    e: Color,
    f: Color,
    bg: Color,
) {
    let colors = [a, b, c, d, e, f];
    for (idx, color) in colors.iter().enumerate() {
        buf.set_string(
            x + idx as u16 * 3,
            y,
            "██",
            Style::default().fg(*color).bg(bg),
        );
    }
}
