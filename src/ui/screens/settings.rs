use super::common::{self, MenuConfig, MenuItem, PanelLayout, SynthwaveBackground};
use crate::{app::screens::SettingsState, ui::view::SettingsViewModel};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

pub fn render_settings(
    frame: &mut Frame,
    area: Rect,
    view: &SettingsViewModel,
    state: &mut SettingsState,
) {
    state.row_areas.clear();
    common::paint_synthwave(frame.buffer_mut(), area, SynthwaveBackground::default());

    render_title(frame.buffer_mut(), area, area.y + 1, view);

    let layout = common::centered_panel(area, 48, 66, 28, 4);
    common::render_bordered_panel(frame, &layout, "OPTIONS");
    render_panel_content(frame, &layout, view, state);

    common::render_footer(
        frame.buffer_mut(),
        area,
        "Arrow Keys Move  •  Enter Select  •  Esc Back  •  Shift+P Cycle",
    );
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16, view: &SettingsViewModel) {
    common::set_centered_string(
        buf,
        area.x,
        y,
        area.width,
        "Settings",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    );

    let (llm_label, llm_color) = match &view.llm_status {
        crate::ui::view::LlmStatus::Active => {
            ("LLM: Active".to_string(), Color::Rgb(100, 220, 100))
        }
        crate::ui::view::LlmStatus::Unavailable => {
            ("LLM: No Model".to_string(), Color::Rgb(255, 170, 80))
        }
        crate::ui::view::LlmStatus::Disabled => ("LLM: Off".to_string(), Color::Rgb(140, 140, 140)),
        crate::ui::view::LlmStatus::Downloading(_) => {
            ("LLM: Downloading...".to_string(), Color::Rgb(100, 180, 255))
        }
        crate::ui::view::LlmStatus::DownloadFailed(_) => (
            "LLM: Download Failed".to_string(),
            Color::Rgb(255, 100, 100),
        ),
    };
    let full_text = format!("Theme: {}  |  {}", view.current_theme_label, llm_label);
    common::set_centered_string(
        buf,
        area.x,
        y + 1,
        area.width,
        &full_text,
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Reset),
    );
    // Overlay just the LLM portion with its status color
    if let Some(llm_offset) = full_text.find(&*llm_label) {
        let center_x = area.x + area.width.saturating_sub(full_text.len() as u16) / 2;
        let llm_x = center_x + llm_offset as u16;
        buf.set_string(
            llm_x,
            y + 1,
            &llm_label,
            Style::default().fg(llm_color).bg(Color::Reset),
        );
    }
}

fn render_panel_content(
    frame: &mut Frame,
    layout: &PanelLayout,
    view: &SettingsViewModel,
    state: &mut SettingsState,
) {
    let inner = layout.inner;
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    let hint = "Open the palette browser or quick-cycle through themes.";
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            common::truncate(hint, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55)),
    );

    let items: Vec<MenuItem> = view
        .options
        .iter()
        .map(|opt| MenuItem {
            label: opt,
            greyed: false,
        })
        .collect();

    state.row_areas = common::render_menu_items(
        buf,
        inner,
        MenuConfig {
            items: &items,
            selected: view.selected,
            start_y_offset: 2,
            back_button: true,
        },
    );
}
