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

const TOOLBAR_ROWS: u16 = 9;

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

    render_tool_button(
        buf,
        rows[0],
        Tool::Inspect,
        toolbar.current_tool == Tool::Inspect,
        "Query",
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

    render_chooser_button(
        buf,
        rows[2],
        ToolChooserKind::Zones,
        toolbar.zone_tool,
        toolbar.chooser == Some(ToolChooserKind::Zones),
    );
    hit_areas.push(hit_area(
        rows[2],
        ToolbarHitTarget::OpenChooser(ToolChooserKind::Zones),
    ));

    for (row, tool) in rows[3..6]
        .iter()
        .copied()
        .zip([Tool::Road, Tool::Rail, Tool::PowerLine])
    {
        render_tool_button(buf, row, tool, toolbar.current_tool == tool, tool.label());
        hit_areas.push(hit_area(row, ToolbarHitTarget::SelectTool(tool)));
    }

    render_chooser_button(
        buf,
        rows[6],
        ToolChooserKind::PowerPlants,
        toolbar.power_plant_tool,
        toolbar.chooser == Some(ToolChooserKind::PowerPlants),
    );
    hit_areas.push(hit_area(
        rows[6],
        ToolbarHitTarget::OpenChooser(ToolChooserKind::PowerPlants),
    ));

    render_chooser_button(
        buf,
        rows[7],
        ToolChooserKind::Buildings,
        toolbar.building_tool,
        toolbar.chooser == Some(ToolChooserKind::Buildings),
    );
    hit_areas.push(hit_area(
        rows[7],
        ToolbarHitTarget::OpenChooser(ToolChooserKind::Buildings),
    ));

    render_chooser_button(
        buf,
        rows[8],
        ToolChooserKind::Amusement,
        toolbar.amusement_tool,
        toolbar.chooser == Some(ToolChooserKind::Amusement),
    );
    hit_areas.push(hit_area(
        rows[8],
        ToolbarHitTarget::OpenChooser(ToolChooserKind::Amusement),
    ));

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
        Tool::ZoneRes => Some(TaxSector::Residential),
        Tool::ZoneComm => Some(TaxSector::Commercial),
        Tool::ZoneInd => Some(TaxSector::Industrial),
        _ => None,
    }
}

fn tool_key(tool: Tool) -> String {
    format!("[{}]", tool.key_hint().to_ascii_uppercase())
}

fn chooser_key(kind: ToolChooserKind) -> &'static str {
    match kind {
        ToolChooserKind::Zones => "[1-3]",
        ToolChooserKind::PowerPlants => "[E/G]",
        ToolChooserKind::Buildings => "[S/F]",
        ToolChooserKind::Amusement => "[K]",
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
    fn toolbar_height_matches_full_button_stack() {
        let toolbar = ToolbarPaletteViewModel {
            current_tool: Tool::Inspect,
            zone_tool: Tool::ZoneRes,
            power_plant_tool: Tool::PowerPlantCoal,
            building_tool: Tool::Police,
            amusement_tool: Tool::Park,
            chooser: None,
        };

        assert_eq!(toolbar_height(&toolbar), 9);
        assert_eq!(minimum_toolbar_height(&toolbar), 9);
    }

    #[test]
    fn chooser_buttons_map_to_all_split_groups() {
        let toolbar = ToolbarPaletteViewModel {
            current_tool: Tool::Inspect,
            zone_tool: Tool::ZoneRes,
            power_plant_tool: Tool::PowerPlantCoal,
            building_tool: Tool::Police,
            amusement_tool: Tool::Park,
            chooser: Some(ToolChooserKind::Buildings),
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 24, 9));

        let hits = render_toolbar(Rect::new(0, 0, 24, 9), &mut buf, &toolbar);

        assert!(hits
            .iter()
            .any(|hit| hit.target == ToolbarHitTarget::OpenChooser(ToolChooserKind::Zones)));
        assert!(hits
            .iter()
            .any(|hit| hit.target == ToolbarHitTarget::OpenChooser(ToolChooserKind::PowerPlants)));
        assert!(hits
            .iter()
            .any(|hit| hit.target == ToolbarHitTarget::OpenChooser(ToolChooserKind::Buildings)));
        assert!(hits
            .iter()
            .any(|hit| hit.target == ToolbarHitTarget::OpenChooser(ToolChooserKind::Amusement)));
    }
}
