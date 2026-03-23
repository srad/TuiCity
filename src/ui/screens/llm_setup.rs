use super::common::{self, MenuConfig, MenuItem, PanelLayout, SynthwaveBackground};
use crate::{
    app::screens::LlmSetupState,
    ui::frontends::terminal::render_confirm_dialog,
    ui::view::{DownloadProgressViewModel, LlmSetupViewModel},
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

    let layout = common::centered_panel(area, 60, 82, 40, 4);
    common::render_bordered_panel(frame, &layout, "TEXT GENERATION");
    render_panel_content(frame, &layout, view, state);

    common::render_footer(
        frame.buffer_mut(),
        area,
        "Up/Down Move  •  Left/Right Change Model or GPU Mode  •  Enter Select  •  Esc Back",
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

    let (status_text, status_color) = if let Some(progress) = &view.download_progress {
        let message = if progress.cancelling {
            "Canceling download...".to_string()
        } else {
            format!("Downloading {}", view.selected_model_label)
        };
        (message, Color::Rgb(100, 180, 255))
    } else if let Some(error) = &view.download_failed {
        (
            format!("Download failed — {}", common::truncate(error, 48)),
            Color::Rgb(255, 120, 120),
        )
    } else if let Some(notice) = &view.download_notice {
        (notice.clone(), Color::Rgb(255, 190, 110))
    } else if view.backend_status.starts_with("Active") {
        (view.backend_status.clone(), Color::Rgb(100, 220, 100))
    } else {
        (view.backend_status.clone(), Color::Rgb(170, 170, 170))
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
    let info_style = Style::default()
        .fg(Color::Rgb(170, 223, 219))
        .bg(Color::Rgb(35, 34, 55));
    let highlight_style = Style::default()
        .fg(Color::Rgb(255, 221, 119))
        .bg(Color::Rgb(35, 34, 55));
    let muted_style = Style::default()
        .fg(Color::Rgb(190, 190, 200))
        .bg(Color::Rgb(35, 34, 55));

    write_line(
        buf,
        inner,
        0,
        "Choose a local model, then use Left/Right on Model or Acceleration.",
        info_style,
    );
    write_line(
        buf,
        inner,
        1,
        &format!(
            "Selected Model: {} ({})",
            view.selected_model_label, view.selected_model_size_label
        ),
        highlight_style,
    );
    write_line(buf, inner, 2, &view.selected_model_description, muted_style);
    write_line(
        buf,
        inner,
        3,
        &format!(
            "Acceleration: {} — {}",
            view.gpu_mode_label, view.gpu_mode_description
        ),
        info_style,
    );
    write_line(
        buf,
        inner,
        4,
        &format!("GPU Status: {}", view.gpu_status),
        muted_style,
    );

    if let Some(progress) = &view.download_progress {
        write_line(
            buf,
            inner,
            5,
            &render_progress_bar(progress, inner.width.saturating_sub(2) as usize),
            highlight_style,
        );
        write_line(
            buf,
            inner,
            6,
            &render_progress_caption(progress),
            muted_style,
        );
    } else if let Some(error) = &view.download_failed {
        write_line(
            buf,
            inner,
            5,
            &format!("Last error: {}", error),
            Style::default()
                .fg(Color::Rgb(255, 140, 140))
                .bg(Color::Rgb(35, 34, 55)),
        );
    } else if let Some(notice) = &view.download_notice {
        write_line(
            buf,
            inner,
            5,
            notice,
            Style::default()
                .fg(Color::Rgb(255, 190, 110))
                .bg(Color::Rgb(35, 34, 55)),
        );
    } else if view.model_installed {
        write_line(
            buf,
            inner,
            5,
            "Selected model is already installed.",
            muted_style,
        );
    } else {
        write_line(
            buf,
            inner,
            5,
            "Selected model is not installed yet.",
            muted_style,
        );
    }

    let busy = view.download_progress.is_some();

    let toggle_label = if view.llm_enabled {
        "Text Generation: ON".to_string()
    } else {
        "Text Generation: OFF".to_string()
    };
    let model_label = format!("Model: {}", view.selected_model_label);
    let gpu_label = format!("Acceleration: {}", view.gpu_mode_label);
    let download_label = if busy {
        "Download in Progress".to_string()
    } else if view.model_installed {
        "Model Installed".to_string()
    } else {
        format!(
            "Download Selected Model ({})",
            view.selected_model_size_label
        )
    };

    let items = [
        MenuItem {
            label: &toggle_label,
            greyed: busy,
        },
        MenuItem {
            label: &model_label,
            greyed: busy,
        },
        MenuItem {
            label: &gpu_label,
            greyed: busy,
        },
        MenuItem {
            label: &download_label,
            greyed: busy || view.model_installed,
        },
        MenuItem {
            label: "Cancel Download",
            greyed: !busy,
        },
        MenuItem {
            label: "Delete Selected Model Files",
            greyed: busy || !view.model_installed,
        },
    ];

    state.row_areas = common::render_menu_items(
        buf,
        inner,
        MenuConfig {
            items: &items,
            selected: view.selected,
            start_y_offset: 8,
            back_button: true,
        },
    );
}

fn write_line(
    buf: &mut ratatui::buffer::Buffer,
    inner: Rect,
    line_offset: u16,
    text: &str,
    style: Style,
) {
    if line_offset >= inner.height {
        return;
    }
    buf.set_string(
        inner.x,
        inner.y + line_offset,
        format!(
            "{:<width$}",
            common::truncate(text, inner.width as usize),
            width = inner.width as usize
        ),
        style,
    );
}

fn render_progress_bar(progress: &DownloadProgressViewModel, width: usize) -> String {
    let percent = progress.percent.unwrap_or(0) as usize;
    let bar_width = width.saturating_sub(8).clamp(10, 40);
    let filled = bar_width.saturating_mul(percent).saturating_div(100);
    let mut bar = String::from("[");
    for index in 0..bar_width {
        bar.push(if index < filled { '=' } else { '.' });
    }
    bar.push(']');
    if progress.cancelling {
        format!("{bar} canceling")
    } else if progress.percent.is_some() {
        format!("{bar} {:>3}%", percent)
    } else {
        format!("{bar} ...")
    }
}

fn render_progress_caption(progress: &DownloadProgressViewModel) -> String {
    let amount = match progress.total_bytes {
        Some(total) => format!(
            "{} / {}",
            format_bytes(progress.downloaded_bytes),
            format_bytes(total)
        ),
        None => format!("{} downloaded", format_bytes(progress.downloaded_bytes)),
    };
    format!("{} — {}", progress.label, amount)
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.0} MB", bytes / MIB)
    } else {
        format!("{:.0} KB", bytes / 1024.0)
    }
}
