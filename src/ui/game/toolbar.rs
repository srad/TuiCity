use crate::core::tool::Tool;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::StatefulWidget,
};
use rat_widget::button::Button;

pub const TOOL_GROUPS: &[(&str, &[Tool])] = &[
    ("", &[Tool::Inspect, Tool::Bulldoze]),
    ("Zones", &[Tool::ZoneRes, Tool::ZoneComm, Tool::ZoneInd]),
    ("Roads/Power", &[Tool::Road, Tool::Rail, Tool::PowerLine]),
    (
        "Buildings",
        &[Tool::PowerPlant, Tool::Park, Tool::Police, Tool::Fire],
    ),
];

pub fn render_toolbar(
    area: Rect,
    buf: &mut Buffer,
    current_tool: Tool,
    btn_states: &mut std::collections::HashMap<Tool, rat_widget::button::ButtonState>,
) {
    if area.width < 3 || area.height < 2 {
        return;
    }

    // Fill background
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.cell_mut((x, y)).unwrap();
            cell.set_char(' ');
            cell.set_bg(Color::Rgb(20, 20, 35));
        }
    }

    let mut row = area.y;

    for &(group_name, tools) in TOOL_GROUPS {
        if row >= area.y + area.height {
            break;
        }

        // Group header — omitted for unnamed groups
        if !group_name.is_empty() {
            let header = format!("┌ {}", group_name);
            let header_trimmed = truncate(&header, area.width as usize);
            buf.set_string(
                area.x,
                row,
                &header_trimmed,
                Style::default()
                    .fg(Color::Rgb(150, 150, 200))
                    .bg(Color::Rgb(20, 20, 35)),
            );
            let header_char_count = header_trimmed.chars().count() as u16;
            let start_x = area.x + header_char_count;
            if start_x < area.x + area.width {
                let remaining = (area.x + area.width - start_x) as usize;
                let rest: String = std::iter::repeat_n('─', remaining).collect();
                buf.set_string(
                    start_x,
                    row,
                    &rest,
                    Style::default()
                        .fg(Color::Rgb(60, 60, 80))
                        .bg(Color::Rgb(20, 20, 35)),
                );
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
            let cost_str = if cost == 0 {
                String::new()
            } else {
                format!(" (${cost})")
            };
            let btn_text = format!("[{}] {}{}", hint, label, cost_str);
            let btn_text = truncate(&btn_text, (area.width as usize).saturating_sub(1));

            let base_style = Style::default().fg(Color::Rgb(200, 200, 220)).bg(Color::Rgb(30, 30, 50));
            let active_style = Style::default().fg(Color::Black).bg(Color::Rgb(220, 200, 60)).add_modifier(Modifier::BOLD);

            // Use rat_widget::button::Button for standardized event handling & styling
            let mut button = Button::new(btn_text.clone())
                .styles(rat_widget::button::ButtonStyle {
                    style: if is_active { active_style } else { base_style },
                    armed: Some(Style::default().bg(Color::Rgb(255, 100, 0))),
                    ..Default::default()
                });
            
            // If active, we want it to look "selected" even if not focused
            if is_active {
                button = button.style(active_style);
            }

            let btn_area = Rect::new(area.x, row, area.width, 1);
            if let Some(state) = btn_states.get_mut(&tool) {
                button.render(btn_area, buf, state);
            }

            row += 1;
        }

        // Group footer
        if row < area.y + area.height {
            let footer: String = std::iter::repeat_n('─', area.width as usize).collect();
            buf.set_string(
                area.x,
                row,
                &footer,
                Style::default()
                    .fg(Color::Rgb(60, 60, 80))
                    .bg(Color::Rgb(20, 20, 35)),
            );
            row += 1;
        }
    }
}

/// Exact number of rows the toolbar content occupies.
pub fn toolbar_height() -> u16 {
    let mut h = 0u16;
    for &(group_name, tools) in TOOL_GROUPS {
        if !group_name.is_empty() { h += 1; } // header
        h += tools.len() as u16;
        h += 1; // footer separator
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
