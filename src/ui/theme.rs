use crate::core::map::{Tile, TileOverlay};
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
            OverlayMode::Power     => "[Overlay: Power]",
            OverlayMode::Pollution => "[Overlay: Pollution]",
            OverlayMode::LandValue => "[Overlay: Land Value]",
            OverlayMode::Crime     => "[Overlay: Crime]",
            OverlayMode::FireRisk  => "[Overlay: Fire Risk]",
        }
    }
}

/// Returns a background color tint for the given overlay mode and tile overlay.
/// Returns `None` when overlay is `None` (no tinting).
pub fn overlay_tint(mode: OverlayMode, o: TileOverlay) -> Option<Color> {
    match mode {
        OverlayMode::None      => None,
        OverlayMode::Power     => Some(if o.powered {
            Color::Rgb(0, 60, 20)
        } else {
            Color::Rgb(50, 0, 0)
        }),
        OverlayMode::Pollution => Some(lerp_color(o.pollution,  (20, 50, 20), (120, 40, 0))),
        OverlayMode::LandValue => Some(lerp_color(o.land_value, (10, 20, 10), (0, 120, 60))),
        OverlayMode::Crime     => Some(lerp_color(o.crime,      (20, 20, 20), (100, 0, 0))),
        OverlayMode::FireRisk  => Some(lerp_color(o.fire_risk,  (20, 10, 10), (140, 40, 0))),
    }
}

fn lerp_color(val: u8, low: (u8, u8, u8), high: (u8, u8, u8)) -> Color {
    let t = val as f32 / 255.0;
    let r = (low.0 as f32 + t * (high.0 as f32 - low.0 as f32)) as u8;
    let g = (low.1 as f32 + t * (high.1 as f32 - low.1 as f32)) as u8;
    let b = (low.2 as f32 + t * (high.2 as f32 - low.2 as f32)) as u8;
    Color::Rgb(r, g, b)
}

// ── Feature 6: Theme struct ───────────────────────────────────────────────────

/// Centralised asset registry.  `default_theme()` returns the current hardcoded
/// values; swap the struct to change the entire visual style at runtime.
#[allow(dead_code)]
pub struct Theme {
    pub road_chars:  [char; 16],
    pub rail_chars:  [char; 16],
    pub power_chars: [char; 16],
    pub cursor_fg:   Color,
    pub cursor_bg:   Color,
}

#[allow(dead_code)]
impl Theme {
    pub fn default_theme() -> Self {
        Self {
            road_chars:  ROAD_CHARS,
            rail_chars:  RAIL_CHARS,
            power_chars: POWER_CHARS,
            cursor_fg:   Color::Black,
            cursor_bg:   Color::Rgb(255, 220, 0),
        }
    }

    pub fn glyph(&self, tile: Tile, overlay: TileOverlay) -> TileGlyph {
        tile_glyph(tile, overlay)
    }

    pub fn net_char(&self, tile: Tile, n: bool, e: bool, s: bool, w: bool) -> char {
        network_char(tile, n, e, s, w)
    }
}

// ── Tile glyphs ───────────────────────────────────────────────────────────────

pub struct TileGlyph {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

pub fn tile_glyph(tile: Tile, overlay: TileOverlay) -> TileGlyph {
    let base = match tile {
        Tile::Grass => TileGlyph {
            ch: ' ',
            fg: Color::Reset,
            bg: Color::Rgb(34, 120, 34),
        },
        Tile::Water => TileGlyph {
            ch: '≈',
            fg: Color::Rgb(64, 164, 223),
            bg: Color::Rgb(0, 60, 140),
        },
        Tile::Trees => TileGlyph {
            ch: '♠',
            fg: Color::Rgb(0, 90, 0),
            bg: Color::Rgb(20, 100, 20),
        },
        Tile::Dirt => TileGlyph {
            ch: '·',
            fg: Color::Rgb(160, 130, 80),
            bg: Color::Rgb(120, 90, 50),
        },
        Tile::Road => TileGlyph {
            ch: '╬',
            fg: Color::Rgb(200, 200, 200),
            bg: Color::Rgb(70, 70, 70),
        },
        Tile::Rail => TileGlyph {
            ch: '╬',
            fg: Color::Rgb(160, 160, 160),
            bg: Color::Rgb(101, 67, 33),
        },
        Tile::PowerLine => TileGlyph {
            ch: '┼',
            fg: Color::Rgb(255, 220, 0),
            bg: Color::Rgb(40, 40, 40),
        },
        Tile::RoadPowerLine => TileGlyph {
            ch: '┼',
            fg: Color::Rgb(220, 220, 120),
            bg: Color::Rgb(65, 65, 45),
        },
        Tile::ZoneRes => TileGlyph {
            ch: 'R',
            fg: Color::Rgb(100, 200, 100),
            bg: Color::Rgb(20, 60, 20),
        },
        Tile::ZoneComm => TileGlyph {
            ch: 'C',
            fg: Color::Rgb(100, 160, 255),
            bg: Color::Rgb(10, 40, 80),
        },
        Tile::ZoneInd => TileGlyph {
            ch: 'I',
            fg: Color::Rgb(255, 200, 80),
            bg: Color::Rgb(70, 50, 10),
        },
        Tile::ResLow => TileGlyph {
            ch: '▪',
            fg: Color::Rgb(80, 200, 80),
            bg: Color::Rgb(20, 80, 20),
        },
        Tile::ResMed => TileGlyph {
            ch: '▬',
            fg: Color::Rgb(60, 220, 60),
            bg: Color::Rgb(15, 100, 15),
        },
        Tile::ResHigh => TileGlyph {
            ch: '█',
            fg: Color::Rgb(40, 255, 40),
            bg: Color::Rgb(10, 120, 10),
        },
        Tile::CommLow => TileGlyph {
            ch: '▫',
            fg: Color::Rgb(80, 140, 255),
            bg: Color::Rgb(10, 40, 100),
        },
        Tile::CommHigh => TileGlyph {
            ch: '▮',
            fg: Color::Rgb(60, 160, 255),
            bg: Color::Rgb(10, 60, 130),
        },
        Tile::IndLight => TileGlyph {
            ch: '▦',
            fg: Color::Rgb(220, 180, 50),
            bg: Color::Rgb(70, 55, 10),
        },
        Tile::IndHeavy => TileGlyph {
            ch: '▩',
            fg: Color::Rgb(255, 160, 30),
            bg: Color::Rgb(90, 60, 0),
        },
        Tile::PowerPlant => TileGlyph {
            ch: 'Y',
            fg: Color::Rgb(255, 240, 0),
            bg: Color::Rgb(60, 60, 0),
        },
        Tile::Park => TileGlyph {
            ch: '♣',
            fg: Color::Rgb(30, 200, 30),
            bg: Color::Rgb(10, 80, 10),
        },
        Tile::Police => TileGlyph {
            ch: 'P',
            fg: Color::Rgb(40, 80, 255),
            bg: Color::Rgb(10, 20, 80),
        },
        Tile::Fire => TileGlyph {
            ch: 'F',
            fg: Color::Rgb(255, 60, 10),
            bg: Color::Rgb(80, 15, 0),
        },
        Tile::Hospital => TileGlyph {
            ch: '+',
            fg: Color::Rgb(255, 255, 255),
            bg: Color::Rgb(180, 0, 0),
        },
    };

    // Fire overlay overrides background
    if overlay.on_fire {
        return TileGlyph {
            ch: '^',
            fg: Color::Rgb(255, 100, 0),
            bg: Color::Rgb(180, 40, 0),
        };
    }

    base
}

/// Returns a box-drawing character for a multi-tile building tile based on
/// which cardinal neighbors hold the *same* tile type.
/// `n/e/s/w` = true when that neighbor exists and is the same tile.
pub fn building_char(tile: Tile, n: bool, e: bool, s: bool, w: bool) -> char {
    // All four neighbors present → interior cell
    if n && e && s && w {
        return match tile {
            Tile::PowerPlant => 'Y',
            Tile::Park       => '♣',
            Tile::Police     => 'P',
            Tile::Fire       => 'F',
            _                => ' ',
        };
    }
    let top    = !n;
    let bottom = !s;
    let left   = !w;
    let right  = !e;
    match (top, right, bottom, left) {
        // Corners
        (true,  false, false, true)  => '┌',
        (true,  true,  false, false) => '┐',
        (false, false, true,  true)  => '└',
        (false, true,  true,  false) => '┘',
        // Straight edges
        (true,  false, false, false) => '─', // top edge
        (false, false, true,  false) => '─', // bottom edge
        (false, false, false, true)  => '│', // left edge
        (false, true,  false, false) => '│', // right edge
        // Isolated or unusual shape → fallback to symbol
        _ => match tile {
            Tile::PowerPlant => 'Y',
            Tile::Park       => '♣',
            Tile::Police     => 'P',
            Tile::Fire       => 'F',
            _                => '?',
        },
    }
}

pub fn cursor_style() -> (Color, Color) {
    (Color::Black, Color::Rgb(255, 220, 0))
}

/// Returns the correct box-drawing character for a road/rail/powerline tile
/// based on which cardinal neighbors (N, E, S, W) hold a matching tile.
pub fn network_char(tile: Tile, n: bool, e: bool, s: bool, w: bool) -> char {
    let mask = (n as usize) | ((e as usize) << 1) | ((s as usize) << 2) | ((w as usize) << 3);
    match tile {
        Tile::Road | Tile::RoadPowerLine => ROAD_CHARS[mask],
        Tile::Rail => RAIL_CHARS[mask],
        Tile::PowerLine => POWER_CHARS[mask],
        _ => ' ',
    }
}

const ROAD_CHARS: [char; 16] = [
    /* 0000 */ '┼', /* 0001 */ '│', /* 0010 */ '─', /* 0011 */ '└',
    /* 0100 */ '│', /* 0101 */ '│', /* 0110 */ '┌', /* 0111 */ '├',
    /* 1000 */ '─', /* 1001 */ '┘', /* 1010 */ '─', /* 1011 */ '┴',
    /* 1100 */ '┐', /* 1101 */ '┤', /* 1110 */ '┬', /* 1111 */ '┼',
];

const RAIL_CHARS: [char; 16] = [
    /* 0000 */ '╬', /* 0001 */ '║', /* 0010 */ '═', /* 0011 */ '╚',
    /* 0100 */ '║', /* 0101 */ '║', /* 0110 */ '╔', /* 0111 */ '╠',
    /* 1000 */ '═', /* 1001 */ '╝', /* 1010 */ '═', /* 1011 */ '╩',
    /* 1100 */ '╗', /* 1101 */ '╣', /* 1110 */ '╦', /* 1111 */ '╬',
];

const POWER_CHARS: [char; 16] = [
    /* 0000 */ '╋', /* 0001 */ '┃', /* 0010 */ '━', /* 0011 */ '┗',
    /* 0100 */ '┃', /* 0101 */ '┃', /* 0110 */ '┏', /* 0111 */ '┣',
    /* 1000 */ '━', /* 1001 */ '┛', /* 1010 */ '━', /* 1011 */ '┻',
    /* 1100 */ '┓', /* 1101 */ '┫', /* 1110 */ '┳', /* 1111 */ '╋',
];

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::TileOverlay;

    #[test]
    fn overlay_mode_cycles_through_all_variants() {
        let mut mode = OverlayMode::None;
        let sequence = [
            OverlayMode::Power,
            OverlayMode::Pollution,
            OverlayMode::LandValue,
            OverlayMode::Crime,
            OverlayMode::FireRisk,
            OverlayMode::None,
        ];
        for expected in sequence {
            mode = mode.next();
            assert_eq!(mode, expected);
        }
    }

    #[test]
    fn overlay_tint_none_returns_none() {
        let overlay = TileOverlay::default();
        assert!(overlay_tint(OverlayMode::None, overlay).is_none());
    }

    #[test]
    fn overlay_tint_power_powered_is_green() {
        let overlay = TileOverlay { powered: true, ..TileOverlay::default() };
        let color = overlay_tint(OverlayMode::Power, overlay).unwrap();
        assert_eq!(color, Color::Rgb(0, 60, 20));
    }

    #[test]
    fn overlay_tint_power_unpowered_is_red() {
        let overlay = TileOverlay { powered: false, ..TileOverlay::default() };
        let color = overlay_tint(OverlayMode::Power, overlay).unwrap();
        assert_eq!(color, Color::Rgb(50, 0, 0));
    }

    #[test]
    fn lerp_color_zero_returns_low() {
        let c = lerp_color(0, (10, 20, 30), (100, 200, 255));
        assert_eq!(c, Color::Rgb(10, 20, 30));
    }

    #[test]
    fn lerp_color_max_returns_high() {
        let c = lerp_color(255, (10, 20, 30), (100, 200, 255));
        assert_eq!(c, Color::Rgb(100, 200, 255));
    }

    #[test]
    fn theme_default_has_correct_cursor_colors() {
        let theme = Theme::default_theme();
        assert_eq!(theme.cursor_fg, Color::Black);
        assert_eq!(theme.cursor_bg, Color::Rgb(255, 220, 0));
    }
}
