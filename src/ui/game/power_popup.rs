use crate::{
    core::tool::Tool,
    ui::{theme, view::ToolChooserViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

pub fn render_tool_chooser_content(
    buf: &mut Buffer,
    area: Rect,
    chooser: &ToolChooserViewModel,
) -> Vec<(crate::app::ClickArea, Tool)> {
    if area.width < 12 || area.height < 1 {
        return Vec::new();
    }

    let ui = theme::ui_palette();
    let mut hits = Vec::new();

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_bg(ui.popup_bg);
                cell.set_fg(ui.text_primary);
            }
        }
    }

    let mut row = area.y;
    for &tool in &chooser.tools {
        if row >= area.y + area.height {
            break;
        }
        let available = tool.is_available(&chooser.ctx);
        let reason = tool.unavailable_reason(&chooser.ctx);
        let is_selected = available && chooser.selected_tool == tool;

        let style = if !available {
            Style::default().fg(ui.text_dim).bg(ui.popup_bg)
        } else if is_selected {
            Style::default()
                .fg(ui.selection_fg)
                .bg(ui.selection_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui.text_primary).bg(ui.popup_bg)
        };

        let line = if let Some(r) = &reason {
            format!("  {:<14} ({})", popup_label(tool), r)
        } else {
            format!(
                "{} {:<14} {:>6}",
                marker(is_selected),
                popup_label(tool),
                format!("${}", tool.cost())
            )
        };
        buf.set_string(
            area.x,
            row,
            truncate_padded(&line, area.width as usize),
            style,
        );

        if available {
            hits.push((
                crate::app::ClickArea {
                    x: area.x,
                    y: row,
                    width: area.width,
                    height: 1,
                },
                tool,
            ));
        }
        row += 1;
    }

    hits
}

fn marker(is_selected: bool) -> &'static str {
    if is_selected {
        ">"
    } else {
        " "
    }
}

fn popup_label(tool: Tool) -> &'static str {
    match tool {
        Tool::ZoneResLight => "Res Light",
        Tool::ZoneResDense => "Res Dense",
        Tool::ZoneCommLight => "Comm Light",
        Tool::ZoneCommDense => "Comm Dense",
        Tool::ZoneIndLight => "Ind Light",
        Tool::ZoneIndDense => "Ind Dense",
        Tool::PowerPlantCoal => "Coal Plant",
        Tool::PowerPlantGas => "Gas Plant",
        Tool::Park => "Park",
        Tool::Police => "Police",
        Tool::Fire => "Fire Dept",
        _ => tool.label(),
    }
}

fn truncate_padded(text: &str, width: usize) -> String {
    format!("{:<width$}", truncate(text, width), width = width)
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        text.chars().take(max).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{sim::UnlockMode, tool::ToolContext};

    #[test]
    fn chooser_renders_one_hit_area_per_tool() {
        let chooser = ToolChooserViewModel {
            selected_tool: Tool::ZoneResLight,
            tools: vec![Tool::ZoneResLight, Tool::ZoneCommLight, Tool::ZoneIndLight],
            ctx: ToolContext { year: 1900, unlock_mode: UnlockMode::Historical },
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 28, 8));

        let hits = render_tool_chooser_content(&mut buf, Rect::new(0, 0, 28, 8), &chooser);

        // All zone tools have unlock_year() == 0, so all are available
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn chooser_omits_hit_area_for_locked_tool() {
        let chooser = ToolChooserViewModel {
            selected_tool: Tool::PowerPlantNuclear,
            tools: vec![Tool::PowerPlantCoal, Tool::PowerPlantNuclear],
            ctx: ToolContext { year: 1900, unlock_mode: UnlockMode::Historical },
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 28, 8));

        let hits = render_tool_chooser_content(&mut buf, Rect::new(0, 0, 28, 8), &chooser);

        // Coal (unlock_year=0) gets a hit; Nuclear (unlock_year=1955) does not
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, Tool::PowerPlantCoal);
    }

    #[test]
    fn sandbox_mode_unlocks_all_tools() {
        let chooser = ToolChooserViewModel {
            selected_tool: Tool::PowerPlantCoal,
            tools: vec![Tool::PowerPlantCoal, Tool::PowerPlantNuclear, Tool::PowerPlantSolar],
            ctx: ToolContext { year: 1900, unlock_mode: UnlockMode::Sandbox },
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 28, 8));

        let hits = render_tool_chooser_content(&mut buf, Rect::new(0, 0, 28, 8), &chooser);

        assert_eq!(hits.len(), 3);
    }
}
