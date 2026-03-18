use crate::core::{map::{Tile, TileOverlay}, sim::TaxSector};
use ratatui::style::Color;

// ── Feature 1: Overlay mode ───────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum OverlayMode {
    #[default]
    None,
    Power,
    Pollution,
    LandValue,
    Crime,
    FireRisk,
}

impl OverlayMode {
    pub fn next(self) -> Self {
        match self {
            OverlayMode::None      => OverlayMode::Power,
            OverlayMode::Power     => OverlayMode::Pollution,
            OverlayMode::Pollution => OverlayMode::LandValue,
            OverlayMode::LandValue => OverlayMode::Crime,
            OverlayMode::Crime     => OverlayMode::FireRisk,
            OverlayMode::FireRisk  => OverlayMode::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OverlayMode::None      => "",
            OverlayMode::Power     => "[Overlay: Power Grid]",
            OverlayMode::Pollution => "[Overlay: Pollution]",
            OverlayMode::LandValue => "[Overlay: Land Value]",
            OverlayMode::Crime     => "[Overlay: Crime Rate]",
            OverlayMode::FireRisk  => "[Overlay: Fire Risk]",
        }
    }
}

// ── Feature 2: Color Palette ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiPalette {
    pub desktop_bg: Color,
    pub title: Color,
    pub subtitle: Color,
    pub window_bg: Color,
    pub window_border: Color,
    pub window_title: Color,
    pub map_window_bg: Color,
    pub panel_window_bg: Color,
    pub budget_window_bg: Color,
    pub inspect_window_bg: Color,
    pub popup_bg: Color,
    pub popup_border: Color,
    pub popup_title: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub success: Color,
    pub danger: Color,
    pub warning: Color,
    pub info: Color,
    pub sector_residential: Color,
    pub sector_residential_bg: Color,
    pub sector_commercial: Color,
    pub sector_commercial_bg: Color,
    pub sector_industrial: Color,
    pub sector_industrial_bg: Color,
    pub menu_bg: Color,
    pub menu_fg: Color,
    pub menu_focus_bg: Color,
    pub menu_focus_fg: Color,
    pub menu_hotkey: Color,
    pub menu_right: Color,
    pub menu_title: Color,
    pub toolbar_bg: Color,
    pub toolbar_header: Color,
    pub toolbar_rule: Color,
    pub toolbar_button_bg: Color,
    pub toolbar_button_fg: Color,
    pub toolbar_active_bg: Color,
    pub toolbar_active_fg: Color,
    pub toolbar_armed_bg: Color,
    pub status_bg: Color,
    pub status_sep: Color,
    pub status_city: Color,
    pub status_population: Color,
    pub status_date: Color,
    pub status_message: Color,
    pub status_button_run_bg: Color,
    pub status_button_run_fg: Color,
    pub status_button_pause_bg: Color,
    pub status_button_pause_fg: Color,
    pub input_bg: Color,
    pub input_fg: Color,
    pub input_focus_bg: Color,
    pub input_focus_fg: Color,
    pub slider_bg: Color,
    pub slider_water_fg: Color,
    pub slider_water_focus_bg: Color,
    pub slider_trees_fg: Color,
    pub slider_trees_focus_bg: Color,
    pub button_bg: Color,
    pub button_fg: Color,
    pub button_focus_bg: Color,
    pub button_focus_fg: Color,
    pub button_armed_bg: Color,
    pub scrollbar_button_fg: Color,
    pub scrollbar_button_bg: Color,
    pub scrollbar_track_fg: Color,
    pub scrollbar_track_bg: Color,
    pub scrollbar_thumb_fg: Color,
    pub scrollbar_thumb_bg: Color,
    pub scrollbar_corner_fg: Color,
    pub scrollbar_corner_bg: Color,
    pub preview_valid_fg: Color,
    pub preview_valid_bg: Color,
    pub preview_invalid_fg: Color,
    pub preview_invalid_bg: Color,
    pub preview_line_fg: Color,
    pub preview_line_bg: Color,
    pub viewport_outline: Color,
    pub disaster_bg: Color,
    pub disaster_border: Color,
    pub disaster_select_bg: Color,
}

pub fn ui_palette() -> UiPalette {
    UiPalette {
        desktop_bg: Color::Rgb(24, 18, 14),
        title: Color::Rgb(242, 181, 76),
        subtitle: Color::Rgb(157, 187, 170),
        window_bg: Color::Rgb(33, 26, 21),
        window_border: Color::Rgb(150, 122, 83),
        window_title: Color::Rgb(238, 216, 172),
        map_window_bg: Color::Rgb(16, 24, 18),
        panel_window_bg: Color::Rgb(31, 24, 22),
        budget_window_bg: Color::Rgb(29, 23, 20),
        inspect_window_bg: Color::Rgb(25, 27, 24),
        popup_bg: Color::Rgb(38, 28, 22),
        popup_border: Color::Rgb(201, 159, 77),
        popup_title: Color::Rgb(244, 206, 124),
        text_primary: Color::Rgb(236, 223, 196),
        text_secondary: Color::Rgb(196, 182, 154),
        text_muted: Color::Rgb(153, 139, 112),
        text_dim: Color::Rgb(109, 98, 78),
        accent: Color::Rgb(235, 170, 72),
        accent_soft: Color::Rgb(113, 171, 161),
        selection_bg: Color::Rgb(111, 70, 28),
        selection_fg: Color::Rgb(249, 238, 214),
        success: Color::Rgb(137, 199, 115),
        danger: Color::Rgb(212, 103, 92),
        warning: Color::Rgb(234, 189, 92),
        info: Color::Rgb(111, 182, 188),
        sector_residential: Color::Rgb(130, 218, 122),
        sector_residential_bg: Color::Rgb(28, 72, 32),
        sector_commercial: Color::Rgb(110, 196, 232),
        sector_commercial_bg: Color::Rgb(26, 48, 84),
        sector_industrial: Color::Rgb(227, 194, 96),
        sector_industrial_bg: Color::Rgb(78, 62, 24),
        menu_bg: Color::Rgb(204, 184, 149),
        menu_fg: Color::Rgb(31, 19, 10),
        menu_focus_bg: Color::Rgb(92, 58, 25),
        menu_focus_fg: Color::Rgb(248, 236, 212),
        menu_hotkey: Color::Rgb(116, 82, 32),
        menu_right: Color::Rgb(101, 83, 53),
        menu_title: Color::Rgb(86, 61, 28),
        toolbar_bg: Color::Rgb(28, 22, 19),
        toolbar_header: Color::Rgb(222, 196, 145),
        toolbar_rule: Color::Rgb(113, 94, 70),
        toolbar_button_bg: Color::Rgb(50, 38, 30),
        toolbar_button_fg: Color::Rgb(230, 218, 193),
        toolbar_active_bg: Color::Rgb(233, 176, 80),
        toolbar_active_fg: Color::Rgb(30, 18, 8),
        toolbar_armed_bg: Color::Rgb(186, 103, 48),
        status_bg: Color::Rgb(36, 27, 22),
        status_sep: Color::Rgb(108, 91, 68),
        status_city: Color::Rgb(245, 203, 116),
        status_population: Color::Rgb(163, 205, 197),
        status_date: Color::Rgb(214, 202, 179),
        status_message: Color::Rgb(238, 183, 91),
        status_button_run_bg: Color::Rgb(99, 123, 67),
        status_button_run_fg: Color::Rgb(244, 239, 224),
        status_button_pause_bg: Color::Rgb(90, 66, 26),
        status_button_pause_fg: Color::Rgb(243, 215, 136),
        input_bg: Color::Rgb(55, 42, 32),
        input_fg: Color::Rgb(243, 229, 181),
        input_focus_bg: Color::Rgb(224, 189, 101),
        input_focus_fg: Color::Rgb(28, 18, 10),
        slider_bg: Color::Rgb(88, 72, 52),
        slider_water_fg: Color::Rgb(93, 174, 200),
        slider_water_focus_bg: Color::Rgb(109, 96, 73),
        slider_trees_fg: Color::Rgb(118, 176, 101),
        slider_trees_focus_bg: Color::Rgb(98, 92, 62),
        button_bg: Color::Rgb(42, 33, 28),
        button_fg: Color::Rgb(226, 213, 188),
        button_focus_bg: Color::Rgb(223, 177, 86),
        button_focus_fg: Color::Rgb(28, 18, 10),
        button_armed_bg: Color::Rgb(178, 101, 56),
        scrollbar_button_fg: Color::Rgb(34, 23, 13),
        scrollbar_button_bg: Color::Rgb(195, 175, 142),
        scrollbar_track_fg: Color::Rgb(106, 89, 62),
        scrollbar_track_bg: Color::Rgb(45, 35, 29),
        scrollbar_thumb_fg: Color::Rgb(248, 232, 203),
        scrollbar_thumb_bg: Color::Rgb(128, 96, 55),
        scrollbar_corner_fg: Color::Rgb(106, 89, 62),
        scrollbar_corner_bg: Color::Rgb(195, 175, 142),
        preview_valid_fg: Color::Rgb(130, 224, 170),
        preview_valid_bg: Color::Rgb(24, 76, 50),
        preview_invalid_fg: Color::Rgb(255, 159, 145),
        preview_invalid_bg: Color::Rgb(90, 24, 24),
        preview_line_fg: Color::Rgb(142, 208, 222),
        preview_line_bg: Color::Rgb(28, 66, 76),
        viewport_outline: Color::Rgb(243, 208, 117),
        disaster_bg: Color::Rgb(43, 19, 18),
        disaster_border: Color::Rgb(191, 88, 72),
        disaster_select_bg: Color::Rgb(95, 35, 31),
    }
}

pub fn sector_color(sector: TaxSector) -> Color {
    let ui = ui_palette();
    match sector {
        TaxSector::Residential => ui.sector_residential,
        TaxSector::Commercial => ui.sector_commercial,
        TaxSector::Industrial => ui.sector_industrial,
    }
}

pub fn sector_bg(sector: TaxSector) -> Color {
    let ui = ui_palette();
    match sector {
        TaxSector::Residential => ui.sector_residential_bg,
        TaxSector::Commercial => ui.sector_commercial_bg,
        TaxSector::Industrial => ui.sector_industrial_bg,
    }
}

/// Linearly interpolates between two RGB colors based on a u8 value (0-255).
pub fn lerp_color(val: u8, low: (u8, u8, u8), high: (u8, u8, u8)) -> Color {
    let f = val as f32 / 255.0;
    let r = (low.0 as f32 + (high.0 as f32 - low.0 as f32) * f) as u8;
    let g = (low.1 as f32 + (high.1 as f32 - low.1 as f32) * f) as u8;
    let b = (low.2 as f32 + (high.2 as f32 - low.2 as f32) * f) as u8;
    Color::Rgb(r, g, b)
}

/// Returns a background color tint for the given overlay mode and tile overlay.
/// Returns `None` when overlay is `None` (no tinting).
pub fn overlay_tint(mode: OverlayMode, o: TileOverlay) -> Option<Color> {
    match mode {
        OverlayMode::None      => None,
        OverlayMode::Power     => {
            if o.power_level == 0 {
                Some(Color::Rgb(50, 0, 0))
            } else {
                // Gradient from Red (weak) to Green (strong)
                Some(lerp_color(o.power_level, (100, 40, 0), (0, 150, 40)))
            }
        },
        OverlayMode::Pollution => Some(lerp_color(o.pollution,  (20, 50, 20), (120, 40, 0))),
        OverlayMode::LandValue => Some(lerp_color(o.land_value, (10, 20, 10), (0, 120, 60))),
        OverlayMode::Crime     => Some(lerp_color(o.crime,      (20, 20, 20), (100, 0, 0))),
        OverlayMode::FireRisk  => Some(lerp_color(o.fire_risk,  (20, 10, 10), (140, 40, 0))),
    }
}

// ── Feature 3: Tile Assets ────────────────────────────────────────────────────

pub struct TileGlyph {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

pub fn tile_glyph(tile: Tile, overlay: TileOverlay) -> TileGlyph {
    let ui = ui_palette();
    if overlay.on_fire {
        return TileGlyph {
            ch: '🔥',
            fg: Color::Rgb(255, 200, 0),
            bg: Color::Rgb(150, 40, 0),
        };
    }

    match tile {
        Tile::Grass => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(40, 100, 40),
            bg: Color::Rgb(25, 60, 25),
        },
        Tile::Water => TileGlyph {
            ch: '≈',
            fg: Color::Rgb(100, 150, 255),
            bg: Color::Rgb(20, 40, 100),
        },
        Tile::Trees => TileGlyph {
            ch: '♣',
            fg: Color::Rgb(30, 180, 30),
            bg: Color::Rgb(20, 50, 20),
        },
        Tile::Dirt => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(100, 80, 60),
            bg: Color::Rgb(60, 45, 35),
        },
        Tile::Road | Tile::RoadPowerLine => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(180, 180, 180),
            bg: Color::Rgb(40, 40, 45),
        },
        Tile::Rail => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(150, 130, 100),
            bg: Color::Rgb(30, 30, 35),
        },
        Tile::PowerLine => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(255, 255, 100),
            bg: Color::Rgb(30, 30, 40),
        },
        Tile::ZoneRes => TileGlyph {
            ch: '▒',
            fg: ui.sector_residential,
            bg: ui.sector_residential_bg,
        },
        Tile::ZoneComm => TileGlyph {
            ch: '▒',
            fg: ui.sector_commercial,
            bg: ui.sector_commercial_bg,
        },
        Tile::ZoneInd => TileGlyph {
            ch: '▒',
            fg: ui.sector_industrial,
            bg: ui.sector_industrial_bg,
        },
        Tile::ResLow => TileGlyph {
            ch: '⌂',
            fg: ui.sector_residential,
            bg: ui.sector_residential_bg,
        },
        Tile::ResMed => TileGlyph {
            ch: '⌂',
            fg: ui.sector_residential,
            bg: ui.sector_residential_bg,
        },
        Tile::ResHigh => TileGlyph {
            ch: '🏢',
            fg: Color::Rgb(208, 245, 202),
            bg: ui.sector_residential_bg,
        },
        Tile::CommLow => TileGlyph {
            ch: '◊',
            fg: ui.sector_commercial,
            bg: ui.sector_commercial_bg,
        },
        Tile::CommHigh => TileGlyph {
            ch: '⌂',
            fg: Color::Rgb(204, 232, 250),
            bg: ui.sector_commercial_bg,
        },
        Tile::IndLight => TileGlyph {
            ch: '⌂',
            fg: ui.sector_industrial,
            bg: ui.sector_industrial_bg,
        },
        Tile::IndHeavy => TileGlyph {
            ch: '⌂',
            fg: Color::Rgb(245, 225, 172),
            bg: ui.sector_industrial_bg,
        },
        Tile::PowerPlantCoal | Tile::PowerPlantGas => TileGlyph {
            ch: 'Y',
            fg: Color::Rgb(255, 255, 100),
            bg: Color::Rgb(60, 60, 30),
        },
        Tile::Park => TileGlyph {
            ch: '♠',
            fg: Color::Rgb(50, 220, 50),
            bg: Color::Rgb(20, 80, 20),
        },
        Tile::Police => TileGlyph {
            ch: 'P',
            fg: Color::White,
            bg: Color::Rgb(20, 20, 150),
        },
        Tile::Fire => TileGlyph {
            ch: 'F',
            fg: Color::White,
            bg: Color::Rgb(150, 20, 20),
        },
        Tile::Hospital => TileGlyph {
            ch: 'H',
            fg: Color::Rgb(255, 100, 100),
            bg: Color::White,
        },
        Tile::Rubble => TileGlyph {
            ch: '░',
            fg: Color::Rgb(80, 80, 80),
            bg: Color::Rgb(40, 40, 40),
        },
    }
}

// ── Feature 4: Network characters (Borders) ──────────────────────────────────

pub fn network_char(tile: Tile, n: bool, e: bool, s: bool, w: bool) -> char {
    let idx = (if n { 1 } else { 0 })
            | (if e { 2 } else { 0 })
            | (if s { 4 } else { 0 })
            | (if w { 8 } else { 0 });

    match tile {
        Tile::Road | Tile::RoadPowerLine => [
            ' ', '║', '═', '╚', '║', '║', '╔', '╠', '═', '╝', '═', '╩', '╗', '╣', '╦', '╬'
        ][idx],
        Tile::Rail => [
            ' ', '╽', '╼', '┗', '╿', '┃', '┏', '┣', '╾', '┛', '━', '┻', '┓', '┫', '┳', '╋'
        ][idx],
        Tile::PowerLine => [
            ' ', '╏', '╌', '┗', '╏', '┃', '┏', '┣', '╌', '┛', '━', '┻', '┓', '┫', '┳', '╋'
        ][idx],
        _ => ' ',
    }
}

pub fn building_char(tile: Tile, n: bool, e: bool, s: bool, w: bool) -> char {
    let _idx = (if n { 1 } else { 0 })
            | (if e { 2 } else { 0 })
            | (if s { 4 } else { 0 })
            | (if w { 8 } else { 0 });

    match tile {
        Tile::PowerPlantCoal | Tile::PowerPlantGas => 'Y',
        Tile::Park => '♠',
        Tile::Police => 'P',
        Tile::Fire => 'F',
        Tile::Hospital => 'H',
        _ => ' ',
    }
}

// ── Feature 5: Cursor ─────────────────────────────────────────────────────────

pub fn cursor_style() -> (Color, Color) {
    (Color::Black, Color::Rgb(255, 220, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_color_zero_returns_low() {
        let low = (10, 20, 30);
        let high = (100, 200, 250);
        assert_eq!(lerp_color(0, low, high), Color::Rgb(10, 20, 30));
    }

    #[test]
    fn lerp_color_max_returns_high() {
        let low = (10, 20, 30);
        let high = (100, 200, 250);
        assert_eq!(lerp_color(255, low, high), Color::Rgb(100, 200, 250));
    }

    #[test]
    fn overlay_mode_cycles_through_all_variants() {
        let mut m = OverlayMode::None;
        for _ in 0..6 { m = m.next(); }
        assert_eq!(m, OverlayMode::None);
    }

    #[test]
    fn overlay_tint_none_returns_none() {
        assert!(overlay_tint(OverlayMode::None, TileOverlay::default()).is_none());
    }

    #[test]
    fn overlay_tint_power_unpowered_is_red() {
        let overlay = TileOverlay { power_level: 0, ..TileOverlay::default() };
        assert_eq!(overlay_tint(OverlayMode::Power, overlay), Some(Color::Rgb(50, 0, 0)));
    }

    #[test]
    fn overlay_tint_power_powered_is_greenish() {
        let overlay = TileOverlay { power_level: 255, ..TileOverlay::default() };
        assert_eq!(overlay_tint(OverlayMode::Power, overlay), Some(Color::Rgb(0, 150, 40)));
    }

    #[test]
    fn ui_palette_exposes_expected_core_colors() {
        let palette = ui_palette();
        assert_eq!(palette.menu_fg, Color::Rgb(31, 19, 10));
        assert_eq!(palette.toolbar_active_bg, Color::Rgb(233, 176, 80));
        assert_eq!(palette.scrollbar_thumb_bg, Color::Rgb(128, 96, 55));
        assert_eq!(palette.sector_residential, Color::Rgb(130, 218, 122));
        assert_eq!(palette.sector_commercial, Color::Rgb(110, 196, 232));
        assert_eq!(palette.sector_industrial, Color::Rgb(227, 194, 96));
    }
}
