use crate::{core::{sim::TaxSector, tool::Tool}, ui::theme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

pub const TOOL_GROUPS: &[(&str, &[Tool])] = &[
    ("", &[Tool::Inspect, Tool::Bulldoze]),
    ("Zones", &[Tool::ZoneRes, Tool::ZoneComm, Tool::ZoneInd]),
    ("Roads/Power", &[Tool::Road, Tool::Rail, Tool::PowerLine]),
    ("Buildings", &[Tool::PowerPlantPicker, Tool::Park, Tool::Police, Tool::Fire]),
];

fn tool_sector(tool: Tool) -> Option<TaxSector> {
    match tool {
        Tool::ZoneRes => Some(TaxSector::Residential),
        Tool::ZoneComm => Some(TaxSector::Commercial),
        Tool::ZoneInd => Some(TaxSector::Industrial),
        _ => None,
    }
}

pub fn render_toolbar(area: Rect, buf: &mut Buffer, current_tool: Tool) {
    if area.width < 3 || area.height < 2 {
        return;
    }

    let ui = theme::ui_palette();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.cell_mut((x, y)).unwrap();
            cell.set_char(' ');
            cell.set_bg(ui.toolbar_bg);
        }
    }

    let mut row = area.y;
    for &(group_name, tools) in TOOL_GROUPS {
        if row >= area.y + area.height {
            break;
        }

        if !group_name.is_empty() {
            let header = format!("┌ {}", group_name);
            let header_trimmed = truncate(&header, area.width as usize);
            buf.set_string(area.x, row, &header_trimmed, Style::default().fg(ui.toolbar_header).bg(ui.toolbar_bg));
            let header_char_count = header_trimmed.chars().count() as u16;
            let start_x = area.x + header_char_count;
            if start_x < area.x + area.width {
                let remaining = (area.x + area.width - start_x) as usize;
                let rest: String = std::iter::repeat_n('─', remaining).collect();
                buf.set_string(start_x, row, &rest, Style::default().fg(ui.toolbar_rule).bg(ui.toolbar_bg));
            }
            row += 1;
        }

        for &tool in tools {
            if row >= area.y + area.height {
                break;
            }
            let is_active = tool == current_tool;
            let hint = tool.key_hint();
            let label = tool.label();
            let cost = tool.cost();
            let cost_str = if cost == 0 { String::new() } else { format!(" (${cost})") };
            let text = truncate(&format!("[{}] {}{}", hint, label, cost_str), area.width as usize);
            let style = if let Some(sector) = tool_sector(tool) {
                if is_active {
                    Style::default()
                        .fg(ui.selection_fg)
                        .bg(theme::sector_bg(sector))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::sector_color(sector)).bg(ui.toolbar_button_bg)
                }
            } else if is_active {
                Style::default()
                    .fg(ui.toolbar_active_fg)
                    .bg(ui.toolbar_active_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.toolbar_button_fg).bg(ui.toolbar_button_bg)
            };

            let padded = format!("{:<width$}", text, width = area.width as usize);
            buf.set_string(area.x, row, padded, style);
            row += 1;
        }

        if row < area.y + area.height {
            let footer: String = std::iter::repeat_n('─', area.width as usize).collect();
            buf.set_string(area.x, row, &footer, Style::default().fg(ui.toolbar_rule).bg(ui.toolbar_bg));
            row += 1;
        }
    }
}

pub fn tool_at_row(relative_row: u16) -> Option<Tool> {
    let mut row = 0u16;
    for &(group_name, tools) in TOOL_GROUPS {
        if !group_name.is_empty() {
            row += 1;
        }
        for &tool in tools {
            if relative_row == row {
                return Some(tool);
            }
            row += 1;
        }
        row += 1;
    }
    None
}

pub fn toolbar_height() -> u16 {
    let mut h = 0u16;
    for &(group_name, tools) in TOOL_GROUPS {
        if !group_name.is_empty() {
            h += 1;
        }
        h += tools.len() as u16;
        h += 1;
    }
    h
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}
