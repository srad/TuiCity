use ratatui::style::Color;
use crate::core::map::{Tile, TileOverlay};

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
            ch: '⚡',
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
            ch: '🔥',
            fg: Color::Rgb(255, 100, 0),
            bg: Color::Rgb(180, 40, 0),
        };
    }

    base
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
        Tile::Rail      => RAIL_CHARS[mask],
        Tile::PowerLine => POWER_CHARS[mask],
        _               => ' ',
    }
}

const ROAD_CHARS: [char; 16] = [
 /* 0000 */ '┼',
 /* 0001 */ '│',
 /* 0010 */ '─',
 /* 0011 */ '└',
 /* 0100 */ '│',
 /* 0101 */ '│',
 /* 0110 */ '┌',
 /* 0111 */ '├',
 /* 1000 */ '─',
 /* 1001 */ '┘',
 /* 1010 */ '─',
 /* 1011 */ '┴',
 /* 1100 */ '┐',
 /* 1101 */ '┤',
 /* 1110 */ '┬',
 /* 1111 */ '┼',
];

const RAIL_CHARS: [char; 16] = [
 /* 0000 */ '╬',
 /* 0001 */ '║',
 /* 0010 */ '═',
 /* 0011 */ '╚',
 /* 0100 */ '║',
 /* 0101 */ '║',
 /* 0110 */ '╔',
 /* 0111 */ '╠',
 /* 1000 */ '═',
 /* 1001 */ '╝',
 /* 1010 */ '═',
 /* 1011 */ '╩',
 /* 1100 */ '╗',
 /* 1101 */ '╣',
 /* 1110 */ '╦',
 /* 1111 */ '╬',
];

const POWER_CHARS: [char; 16] = [
 /* 0000 */ '╋',
 /* 0001 */ '┃',
 /* 0010 */ '━',
 /* 0011 */ '┗',
 /* 0100 */ '┃',
 /* 0101 */ '┃',
 /* 0110 */ '┏',
 /* 0111 */ '┣',
 /* 1000 */ '━',
 /* 1001 */ '┛',
 /* 1010 */ '━',
 /* 1011 */ '┻',
 /* 1100 */ '┓',
 /* 1101 */ '┫',
 /* 1110 */ '┳',
 /* 1111 */ '╋',
];
