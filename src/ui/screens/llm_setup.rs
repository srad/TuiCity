use super::common::{self, MenuConfig, MenuItem, PanelLayout, SynthwaveBackground};
use crate::{
    app::screens::LlmSetupState,
    ui::frontends::terminal::render_confirm_dialog,
    ui::view::LlmSetupViewModel,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

pub fn render_llm_setup(
    frame: &mut Frame,
    area: Rect,
    view: &LlmSetupViewModel,
    state: &mut LlmSetupState,
) {
    state.row_areas.clear();
    common::paint_synthwave(frame.buffer_mut(), area, SynthwaveBackground::default());

    render_title(frame.buffer_mut(), area, area.y + 1, view);

    let layout = common::centered_panel(area, 48, 66, 24, 4);
    common::render_bordered_panel(frame, &layout, "TEXT GENERATION");
    render_panel_content(frame, &layout, view, state);

    common::render_footer(
        frame.buffer_mut(),
        area,
        "Arrow Keys Move  •  Enter Select  •  Esc Back",
    );

    if let Some(dialog) = &view.confirm_dialog {
        render_confirm_dialog(frame, area, dialog);
    }
}

fn render_title(buf: &mut ratatui::buffer::Buffer, area: Rect, y: u16, view: &LlmSetupViewModel) {
    common::set_centered_string(
        buf,
        area.x,
        y,
        area.width,
        "LLM Setup",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    );

    let (status_text, status_color) = if view.download_progress.is_some() {
        ("Downloading...".to_string(), Color::Rgb(100, 180, 255))
    } else if view.model_installed && view.llm_enabled {
        (
            "Active — llama.cpp".to_string(),
            Color::Rgb(100, 220, 100),
        )
    } else if view.model_installed {
        (
            "Model ready — disabled".to_string(),
            Color::Rgb(255, 170, 80),
        )
    } else {
        (
            "No model — using static text".to_string(),
            Color::Rgb(170, 170, 170),
        )
    };

    common::set_centered_string(
        buf,
        area.x,
        y + 1,
        area.width,
        &status_text,
        Style::default().fg(status_color).bg(Color::Reset),
    );
}

fn render_panel_content(
    frame: &mut Frame,
    layout: &PanelLayout,
    view: &LlmSetupViewModel,
    state: &mut LlmSetupState,
) {
    let inner = layout.inner;
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    let desc = "Configure the AI text generation backend.";
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            common::truncate(desc, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55)),
    );

    // Build option labels
    let toggle_label = if view.llm_enabled {
        "Text Generation: ON".to_string()
    } else {
        "Text Generation: OFF".to_string()
    };

    let download_label = if view.download_progress.is_some() {
        let what = view.download_progress.as_deref().unwrap_or("...");
        format!(
            "Downloading: {}",
            common::truncate(what, inner.width.saturating_sub(16) as usize)
        )
    } else if let Some(err) = &view.download_failed {
        format!(
            "Download Failed: {} (retry)",
            common::truncate(err, inner.width.saturating_sub(24) as usize)
        )
    } else if view.model_installed {
        "Model Installed".to_string()
    } else {
        "Download Model (~380 MB)".to_string()
    };

    let delete_label = if view.model_installed {
        "Delete Model Files"
    } else {
        "Delete Model Files (no files)"
    };

    let items = [
        MenuItem {
            label: &toggle_label,
            greyed: false,
        },
        MenuItem {
            label: &download_label,
            greyed: view.model_installed && view.download_progress.is_none(),
        },
        MenuItem {
            label: delete_label,
            greyed: !view.model_installed,
        },
    ];

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
