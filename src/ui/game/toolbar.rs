use crate::{
    core::{sim::TaxSector, tool::Tool},
    ui::{
        runtime::{ClickArea, ToolChooserKind, ToolbarHitArea, ToolbarHitTarget},
        theme,
        view::ToolbarPaletteViewModel,
    },
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

const TOOLBAR_ROWS: u16 = 8;

pub fn toolbar_height(_: &ToolbarPaletteViewModel) -> u16 {
    TOOLBAR_ROWS
}

pub fn minimum_toolbar_height(_: &ToolbarPaletteViewModel) -> u16 {
    TOOLBAR_ROWS
}

pub fn render_toolbar(
    area: Rect,
    buf: &mut Buffer,
    toolbar: &ToolbarPaletteViewModel,
) -> Vec<ToolbarHitArea> {
    if area.width < 14 || area.height < TOOLBAR_ROWS {
        return Vec::new();
    }

    let ui = theme::ui_palette();
    let mut hit_areas = Vec::new();

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_bg(ui.toolbar_bg);
                cell.set_fg(ui.toolbar_button_fg);
            }
        }
    }

    let rows = row_rects(area, TOOLBAR_ROWS);

    let inspect_label = match toolbar.view_layer {
        crate::core::map::ViewLayer::Surface => "Inspect Surface",
        crate::core::map::ViewLayer::Underground => "Inspect Underground",
    };
    render_tool_button(
        buf,
        rows[0],
        Tool::Inspect,
        toolbar.current_tool == Tool::Inspect,
        inspect_label,
    );
    hit_areas.push(hit_area(
        rows[0],
        ToolbarHitTarget::SelectTool(Tool::Inspect),
    ));

    render_tool_button(
        buf,
        rows[1],
        Tool::Bulldoze,
        toolbar.current_tool == Tool::Bulldoze,
        "Bulldoze",
    );
    hit_areas.push(hit_area(
        rows[1],
        ToolbarHitTarget::SelectTool(Tool::Bulldoze),
    ));

    let chooser_rows = [
        (rows[2], ToolChooserKind::Zones, toolbar.zone_tool),
        (rows[3], ToolChooserKind::Transport, toolbar.transport_tool),
        (rows[4], ToolChooserKind::Utilities, toolbar.utility_tool),
        (
            rows[5],
            ToolChooserKind::PowerPlants,
            toolbar.power_plant_tool,
        ),
        (rows[6], ToolChooserKind::Buildings, toolbar.building_tool),
        (rows[7], ToolChooserKind::Terrain, toolbar.terrain_tool),
    ];

    for (row, kind, selected_tool) in chooser_rows {
        render_chooser_button(buf, row, kind, selected_tool, toolbar.chooser == Some(kind));
        hit_areas.push(hit_area(row, ToolbarHitTarget::OpenChooser(kind)));
    }

    hit_areas
}

fn render_chooser_button(
    buf: &mut Buffer,
    area: Rect,
    kind: ToolChooserKind,
    selected_tool: Tool,
    is_open: bool,
) {
    let ui = theme::ui_palette();
    let style = if is_open {
        Style::default()
            .fg(ui.selection_fg)
            .bg(ui.selection_bg)
            .add_modifier(Modifier::BOLD)
    } else if toolbar_tool_matches(kind, selected_tool) {
        chooser_style(selected_tool, true)
    } else {
        Style::default()
            .fg(ui.toolbar_button_fg)
            .bg(ui.toolbar_button_bg)
    };
    let suffix = if is_open {
        "Close"
    } else {
        selected_tool.label()
    };
    let text = format!("{} {}: {}", chooser_key(kind), kind.button_label(), suffix);
    buf.set_string(
        area.x,
        area.y,
        format!(
            "{:<width$}",
            truncate(&text, area.width as usize),
            width = area.width as usize
        ),
        style,
    );
}

fn render_tool_button(buf: &mut Buffer, area: Rect, tool: Tool, is_active: bool, label: &str) {
    let style = chooser_style(tool, is_active);
    let text = format!("{} {}", tool_key(tool), label);
    buf.set_string(
        area.x,
        area.y,
        format!(
            "{:<width$}",
            truncate(&text, area.width as usize),
            width = area.width as usize
        ),
        style,
    );
}

fn chooser_style(tool: Tool, is_active: bool) -> Style {
    let ui = theme::ui_palette();
    if let Some(sector) = tool_sector(tool) {
        if is_active {
            Style::default()
                .fg(ui.selection_fg)
                .bg(theme::sector_bg(sector))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme::sector_color(sector))
                .bg(ui.toolbar_button_bg)
        }
    } else if is_active {
        Style::default()
            .fg(ui.toolbar_active_fg)
            .bg(ui.toolbar_active_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(ui.toolbar_button_fg)
            .bg(ui.toolbar_button_bg)
    }
}

fn toolbar_tool_matches(kind: ToolChooserKind, tool: Tool) -> bool {
    ToolChooserKind::for_tool(tool) == Some(kind)
}

fn row_rects(area: Rect, count: u16) -> Vec<Rect> {
    (0..count)
        .map(|index| Rect::new(area.x, area.y + index, area.width, 1))
        .collect()
}

fn hit_area(area: Rect, target: ToolbarHitTarget) -> ToolbarHitArea {
    ToolbarHitArea {
        area: ClickArea {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        },
        target,
    }
}

fn tool_sector(tool: Tool) -> Option<TaxSector> {
    match tool {
        Tool::ZoneResLight | Tool::ZoneResDense => Some(TaxSector::Residential),
        Tool::ZoneCommLight | Tool::ZoneCommDense => Some(TaxSector::Commercial),
        Tool::ZoneIndLight | Tool::ZoneIndDense => Some(TaxSector::Industrial),
        _ => None,
    }
}

fn tool_key(tool: Tool) -> String {
    format!("[{}]", tool.key_hint().to_ascii_uppercase())
}

fn chooser_key(kind: ToolChooserKind) -> &'static str {
    match kind {
        ToolChooserKind::Zones => "[1-6]",
        ToolChooserKind::Transport => "[R/H/O/L]",
        ToolChooserKind::Utilities => "[P/W/M]",
        ToolChooserKind::PowerPlants => "[E/G/N/V/O]",
        ToolChooserKind::Buildings => "[S/F/K/H/J/X/Q]",
        ToolChooserKind::Terrain => "[W/L/T]",
    }
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
    use crate::ui::view::ToolbarPaletteViewModel;

    #[test]
    fn toolbar_renders_hit_areas_for_new_choosers() {
        let toolbar = ToolbarPaletteViewModel {
            current_tool: Tool::Inspect,
            zone_tool: Tool::ZoneResLight,
            transport_tool: Tool::Road,
            utility_tool: Tool::PowerLine,
            power_plant_tool: Tool::PowerPlantCoal,
            building_tool: Tool::Police,
            terrain_tool: Tool::TerrainWater,
            chooser: None,
            view_layer: crate::core::map::ViewLayer::Surface,
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 26, 10));

        let hits = render_toolbar(Rect::new(0, 0, 26, 10), &mut buf, &toolbar);

        assert!(hits
            .iter()
            .any(|hit| hit.target == ToolbarHitTarget::OpenChooser(ToolChooserKind::Transport)));
        assert!(hits
            .iter()
            .any(|hit| hit.target == ToolbarHitTarget::OpenChooser(ToolChooserKind::Utilities)));
    }
}
