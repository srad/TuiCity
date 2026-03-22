use crate::core::map::{Tile, TileOverlay};

// ─── Public entry point ───────────────────────────────────────────────────────

/// `road_bits`: connectivity flags for Road tiles — N=bit0, E=bit1, S=bit2, W=bit3.
/// Pass 0 for all non-road tiles (ignored).
pub fn draw_tile(
    buf: &mut [u32],
    buf_width: u32,
    px: u32,
    py: u32,
    tile: Tile,
    overlay: &TileOverlay,
    scale: u32,
    fire_ph: u32,
    traffic_ph: u32,
    util_ph: u32,
    blink: bool,
    road_bits: u8,
) {
    let road_storage;
    let pixels: &[u32; 64] = if matches!(tile, Tile::Road) {
        road_storage = road_tile_pixels(road_bits);
        &road_storage
    } else {
        tile_pixels(tile)
    };
    let is_road = matches!(tile, Tile::Road | Tile::Highway | Tile::Onramp | Tile::RoadPowerLine);
    let is_power = matches!(tile, Tile::PowerLine | Tile::RoadPowerLine);

    for row in 0..8u32 {
        for col in 0..8u32 {
            let mut color = pixels[(row * 8 + col) as usize];

            // ── Fire animation ──────────────────────────────────────────────
            if overlay.on_fire {
                // Alternate two offset checkerboard patterns on each phase
                let shifted = (row + col + fire_ph) % 4;
                let c = if shifted < 2 { 0xFF5010u32 } else { 0xCC2000u32 };
                color = lerp(color, c, 180);
            }

            // ── Utility pulse (power lines) ─────────────────────────────────
            if is_power && overlay.power_level > 0 && util_ph % 3 == 0 {
                // Brighten silver wire pixels
                if color == PL2 {
                    color = lerp(PL2, 0xFFFFFF, 80);
                }
            }

            // ── Traffic dot (roads only) ────────────────────────────────────
            if is_road && overlay.traffic > 20 {
                let dot = traffic_ph % 8;
                // Two lanes moving in opposite directions (rows 2 and 5)
                let is_dot =
                    (row == 2 && col == dot) || (row == 5 && col == 7u32.saturating_sub(dot));
                if is_dot {
                    let tc = if overlay.traffic >= 160 { 0xFFEEAAu32 } else { 0xFFD070u32 };
                    color = tc;
                }
            }

            for sy in 0..scale {
                for sx in 0..scale {
                    let x = px + col * scale + sx;
                    let y = py + row * scale + sy;
                    let idx = (y * buf_width + x) as usize;
                    if let Some(p) = buf.get_mut(idx) {
                        *p = color;
                    }
                }
            }
        }
    }

    // ── Plant degradation blink ─────────────────────────────────────────────
    if matches!(tile, Tile::PowerPlantCoal | Tile::PowerPlantGas)
        && overlay.plant_efficiency < 255
        && blink
    {
        // Draw a small amber "!" in the top-centre of the tile (pixels 3,0 to 3,4)
        let amber = 0xFFA020u32;
        let offsets: &[(u32, u32)] = &[(3, 0), (3, 1), (3, 2), (3, 4)]; // col 3, rows 0-2 + row 4
        for &(c, r) in offsets {
            for sy in 0..scale {
                for sx in 0..scale {
                    let x = px + c * scale + sx;
                    let y = py + r * scale + sy;
                    let idx = (y * buf_width + x) as usize;
                    if let Some(p) = buf.get_mut(idx) {
                        *p = amber;
                    }
                }
            }
        }
    }
}

#[inline]
pub fn lerp(a: u32, b: u32, t: u8) -> u32 {
    let t = t as u32;
    let s = 255 - t;
    let r = ((a >> 16 & 0xff) * s + (b >> 16 & 0xff) * t) / 255;
    let g = ((a >> 8 & 0xff) * s + (b >> 8 & 0xff) * t) / 255;
    let bl = ((a & 0xff) * s + (b & 0xff) * t) / 255;
    (r << 16) | (g << 8) | bl
}

/// Generate a road tile from N/E/S/W connectivity bits (N=0, E=1, S=2, W=3).
fn road_tile_pixels(bits: u8) -> [u32; 64] {
    let n = bits & 1 != 0;
    let e = bits & 2 != 0;
    let s = bits & 4 != 0;
    let w = bits & 8 != 0;
    let mut pix = [RD0; 64];
    for row in 0..8u32 {
        for col in 0..8u32 {
            pix[(row * 8 + col) as usize] = road_pixel(row, col, n, e, s, w);
        }
    }
    pix
}

const RD5: u32 = 0x585858; // sidewalk/curb detail
const RD6: u32 = 0x3C3C3C; // asphalt mid-tone

/// Per-pixel colour for a connectivity-aware road tile.
fn road_pixel(row: u32, col: u32, n: bool, e: bool, s: bool, w: bool) -> u32 {
    let on_n = row == 0;
    let on_s = row == 7;
    let on_w = col == 0;
    let on_e = col == 7;

    // Corners: open if either adjoining edge is connected
    if on_n && on_w { return if n || w { RD6 } else { RD4 }; }
    if on_n && on_e { return if n || e { RD6 } else { RD4 }; }
    if on_s && on_w { return if s || w { RD6 } else { RD4 }; }
    if on_s && on_e { return if s || e { RD6 } else { RD4 }; }

    // Edges: kerb on closed side, asphalt on open side
    if on_n { return if n { RD6 } else { RD4 }; }
    if on_s { return if s { RD6 } else { RD4 }; }
    if on_w { return if w { RD6 } else { RD1 }; }
    if on_e { return if e { RD6 } else { RD1 }; }

    // Curb detail strips (row/col 1 or 6) adjacent to a closed edge
    if row == 1 && !n { return RD5; }
    if row == 6 && !s { return RD5; }
    if col == 1 && !w { return RD2; }
    if col == 6 && !e { return RD2; }

    // EW lane markings — dashed yellow centre line
    let has_ew = e || w;
    let has_ns = n || s;

    // Yellow centre dashes (dashed pattern: cols 2-3 on, col 4 off, cols 5-6 on)
    if has_ew && (row == 3 || row == 4) {
        if col == 3 || col == 4 { return RD3; }
    }
    // NS centre dashes
    if has_ns && (col == 3 || col == 4) {
        if row == 3 || row == 4 { return RD3; }
    }

    // White lane edge markings for roads with EW traffic
    if has_ew && (row == 1 || row == 6) && col >= 2 && col <= 5 {
        return RD1; // lighter asphalt at lane edges
    }
    // White lane edge markings for roads with NS traffic
    if has_ns && (col == 1 || col == 6) && row >= 2 && row <= 5 {
        return RD1;
    }

    // Intersection fill: if both NS and EW, fill with asphalt
    if has_ns && has_ew { return RD0; }

    // Default asphalt with subtle variation
    if (row + col) % 2 == 0 { RD0 } else { RD6 }
}

fn tile_pixels(tile: Tile) -> &'static [u32; 64] {
    match tile {
        Tile::Grass => &GRASS,
        Tile::Water => &WATER,
        Tile::Trees => &TREES,
        Tile::Dirt => &DIRT,
        Tile::Road => &ROAD,
        Tile::RoadPowerLine => &ROAD_POWERLINE,
        Tile::Rail => &RAIL,
        Tile::PowerLine => &POWER_LINE,
        Tile::Highway => &HIGHWAY,
        Tile::Onramp => &ONRAMP,
        Tile::WaterPipe => &WATER_PIPE,
        Tile::SubwayTunnel => &SUBWAY_TUNNEL,
        Tile::ZoneRes => &ZONE_RES,
        Tile::ZoneComm => &ZONE_COMM,
        Tile::ZoneInd => &ZONE_IND,
        Tile::ResLow => &RES_LOW,
        Tile::ResMed => &RES_MED,
        Tile::ResHigh => &RES_HIGH,
        Tile::CommLow => &COMM_LOW,
        Tile::CommHigh => &COMM_HIGH,
        Tile::IndLight => &IND_LIGHT,
        Tile::IndHeavy => &IND_HEAVY,
        Tile::PowerPlantCoal => &COAL_PLANT,
        Tile::PowerPlantGas => &GAS_PLANT,
        Tile::Park => &PARK,
        Tile::Police => &POLICE,
        Tile::Fire => &FIRE_DEPT,
        Tile::Hospital => &HOSPITAL,
        Tile::BusDepot => &BUS_DEPOT,
        Tile::RailDepot => &RAIL_DEPOT,
        Tile::SubwayStation => &SUBWAY_STATION,
        Tile::WaterPump => &WATER_PUMP,
        Tile::WaterTower => &WATER_TOWER,
        Tile::WaterTreatment => &WATER_TREATMENT,
        Tile::Desalination => &DESALINATION,
        Tile::Rubble => &RUBBLE,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DESIGN RULES
//  • Terrain: ordered checkerboard dither between two base colours + accent dots
//  • Water:   diagonal ripple sweeping NW→SE, 5 tones
//  • Trees:   concentric circular gradient, darkest at edge, lightest at crown
//  • Buildings: consistent 8×8 frame —
//      row 0/7 & col 0/7  = zone background (never the building itself)
//      row 1   = top highlight strip (NW light source)
//      col 1   = left highlight strip
//      row 6   = bottom shadow strip
//      col 6   = right shadow strip
//      rows 2-5, cols 2-5 = 4×4 interior (roof + detail)
//  • All colours are vivid, SC2000-palette-inspired
// ═══════════════════════════════════════════════════════════════════════════════

// ─── Terrain ──────────────────────────────────────────────────────────────────

const G0: u32 = 0x226018; // dark grass
const G1: u32 = 0x358824; // base grass
const G2: u32 = 0x4AAF30; // light grass
const G3: u32 = 0x60CC40; // bright highlight

// Ordered 2-tone dither: alternating G1/G2 with rare G0/G3 accents
#[rustfmt::skip]
const GRASS: [u32; 64] = [
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,G2,G3,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,G0,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G3,G1,G2,
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G0,
];

const WA0: u32 = 0x002878; // abyss
const WA1: u32 = 0x1050A8; // deep
const WA2: u32 = 0x2878C8; // mid
const WA3: u32 = 0x58A8E0; // light
const WA4: u32 = 0xA8D8F8; // foam

// NW→SE diagonal ripple, 8-pixel period
#[rustfmt::skip]
const WATER: [u32; 64] = [
    WA2,WA1,WA0,WA1,WA3,WA2,WA1,WA0,
    WA1,WA2,WA1,WA0,WA2,WA3,WA2,WA1,
    WA0,WA1,WA3,WA2,WA1,WA2,WA3,WA2,
    WA3,WA0,WA2,WA4,WA2,WA1,WA2,WA3,
    WA2,WA3,WA1,WA2,WA3,WA4,WA1,WA2,
    WA1,WA2,WA3,WA1,WA4,WA2,WA3,WA1,
    WA0,WA1,WA2,WA4,WA1,WA3,WA2,WA4,
    WA1,WA0,WA1,WA2,WA3,WA1,WA4,WA2,
];

const TR0: u32 = 0x0A3006; // ground shadow (between canopies)
const TR1: u32 = 0x175010; // canopy edge dark
const TR2: u32 = 0x267020; // canopy mid
const TR3: u32 = 0x389030; // canopy light
const TR4: u32 = 0x50B040; // crown highlight (sun)

// Concentric rings: darkest at edges, brightest at top-centre
#[rustfmt::skip]
const TREES: [u32; 64] = [
    TR0,TR0,TR1,TR2,TR2,TR1,TR0,TR0,
    TR0,TR1,TR2,TR3,TR3,TR2,TR1,TR0,
    TR1,TR2,TR3,TR4,TR4,TR3,TR2,TR1,
    TR1,TR2,TR4,TR4,TR3,TR4,TR2,TR1,
    TR1,TR2,TR3,TR4,TR4,TR3,TR2,TR1,
    TR1,TR2,TR3,TR3,TR3,TR2,TR2,TR1,
    TR0,TR1,TR2,TR2,TR2,TR2,TR1,TR0,
    TR0,TR0,TR1,TR1,TR1,TR1,TR0,TR0,
];

const DT0: u32 = 0x5A3C10;
const DT1: u32 = 0x7A5820;
const DT2: u32 = 0x9A7030;
const DT3: u32 = 0xBA8A40;

#[rustfmt::skip]
const DIRT: [u32; 64] = [
    DT2,DT1,DT2,DT1,DT2,DT1,DT2,DT1,
    DT1,DT2,DT1,DT2,DT1,DT2,DT1,DT2,
    DT2,DT1,DT3,DT2,DT1,DT2,DT1,DT2,
    DT1,DT2,DT1,DT2,DT3,DT1,DT2,DT1,
    DT2,DT0,DT2,DT1,DT2,DT1,DT2,DT1,
    DT1,DT2,DT1,DT2,DT1,DT2,DT0,DT2,
    DT2,DT1,DT2,DT3,DT2,DT1,DT2,DT1,
    DT1,DT2,DT1,DT2,DT1,DT2,DT1,DT2,
];

// ─── Infrastructure ───────────────────────────────────────────────────────────

const RD0: u32 = 0x303030; // dark asphalt
const RD1: u32 = 0x484848; // asphalt lighter (lane fill)
const RD2: u32 = 0x787878; // kerb/shoulder grey
const RD3: u32 = 0xD4D020; // yellow centre dash
const RD4: u32 = 0xA0A0A0; // kerb edge highlight

// Two-lane top-down road: yellow centre dashes, kerb edges at top/bottom
#[rustfmt::skip]
const ROAD: [u32; 64] = [
    RD4,RD4,RD4,RD4,RD4,RD4,RD4,RD4,  // top kerb edge
    RD2,RD0,RD1,RD3,RD3,RD1,RD0,RD2,  // lane + centre dash
    RD2,RD0,RD1,RD0,RD0,RD1,RD0,RD2,  // lane, no dash
    RD2,RD0,RD1,RD3,RD3,RD1,RD0,RD2,  // lane + centre dash
    RD2,RD0,RD1,RD3,RD3,RD1,RD0,RD2,  // lane + centre dash
    RD2,RD0,RD1,RD0,RD0,RD1,RD0,RD2,  // lane, no dash
    RD2,RD0,RD1,RD3,RD3,RD1,RD0,RD2,  // lane + centre dash
    RD4,RD4,RD4,RD4,RD4,RD4,RD4,RD4,  // bottom kerb edge
];

const PL0: u32 = 0xA07848; // pole wood
const PL1: u32 = 0xC8A060; // pole light
const PL2: u32 = 0xD8D8D8; // wire silver

// Road with power pole at column 2-3; wire at rows 0 and 7
#[rustfmt::skip]
const ROAD_POWERLINE: [u32; 64] = [
    PL2,RD4,PL1,PL1,RD4,RD4,RD4,PL2,  // top kerb + pole cap + wire
    RD2,RD0,PL0,RD3,RD3,RD1,RD0,RD2,  // lane + pole + dash
    RD2,RD0,PL0,RD0,RD0,RD1,RD0,RD2,  // lane + pole
    RD2,RD0,PL0,RD3,RD3,RD1,RD0,RD2,  // lane + pole + dash
    RD2,RD0,PL0,RD3,RD3,RD1,RD0,RD2,  // lane + pole + dash
    RD2,RD0,PL0,RD0,RD0,RD1,RD0,RD2,  // lane + pole
    RD2,RD0,PL0,RD3,RD3,RD1,RD0,RD2,  // lane + pole + dash
    PL2,RD4,PL1,PL1,RD4,RD4,RD4,PL2,  // bottom kerb + pole cap + wire
];

const BA0: u32 = 0x5C4820; // ballast dark
const BA1: u32 = 0x7A6438; // ballast
const BA2: u32 = 0x3E3020; // tie dark
const BA3: u32 = 0x624A2C; // tie
const BA4: u32 = 0xE8E8F0; // steel rail bright
const BA5: u32 = 0xA8A8B8; // steel rail shadow

#[rustfmt::skip]
const RAIL: [u32; 64] = [
    BA1,BA0,BA1,BA0,BA1,BA0,BA1,BA0,
    BA4,BA3,BA2,BA3,BA2,BA3,BA2,BA5,
    BA0,BA1,BA0,BA1,BA0,BA1,BA0,BA1,
    BA0,BA1,BA0,BA1,BA0,BA1,BA0,BA1,
    BA1,BA0,BA1,BA0,BA1,BA0,BA1,BA0,
    BA4,BA3,BA2,BA3,BA2,BA3,BA2,BA5,
    BA0,BA1,BA0,BA1,BA0,BA1,BA0,BA1,
    BA0,BA1,BA0,BA1,BA0,BA1,BA0,BA1,
];

#[rustfmt::skip]
const POWER_LINE: [u32; 64] = [
    G1,G2,G1,PL1,G1,G2,G1,G2,
    PL2,G1,G2,PL0,G2,G1,G2,PL2,
    G2,G1,G2,PL0,G1,G2,G1,G2,
    G1,G2,G1,PL0,G2,G1,G2,G1,
    G2,G1,G2,PL0,G1,G2,G1,G2,
    G1,G2,G1,PL1,G2,G1,G2,G1,
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
];

const HW0: u32 = 0x1A1A1A; // very dark asphalt
const HW1: u32 = 0x282828; // dark asphalt
const HW2: u32 = 0x383838; // asphalt
const HW3: u32 = 0xE0E0E0; // white lane marking
const HW4: u32 = 0xB8B8B8; // concrete barrier
const HW5: u32 = 0xD0D0D0; // barrier highlight

// Highway: wider, darker, white dashes, concrete barriers at edges
#[rustfmt::skip]
const HIGHWAY: [u32; 64] = [
    HW5,HW4,HW4,HW4,HW4,HW4,HW4,HW5,  // barrier top
    HW4,HW0,HW2,HW3,HW3,HW2,HW0,HW4,  // lane + white dash
    HW4,HW0,HW2,HW0,HW0,HW2,HW0,HW4,  // lane, no dash
    HW4,HW0,HW1,HW3,HW3,HW1,HW0,HW4,  // lane + white dash
    HW4,HW0,HW1,HW3,HW3,HW1,HW0,HW4,  // lane + white dash
    HW4,HW0,HW2,HW0,HW0,HW2,HW0,HW4,  // lane, no dash
    HW4,HW0,HW2,HW3,HW3,HW2,HW0,HW4,  // lane + white dash
    HW5,HW4,HW4,HW4,HW4,HW4,HW4,HW5,  // barrier bottom
];

// Onramp: left half=highway, right half=road, diagonal blend in centre
#[rustfmt::skip]
const ONRAMP: [u32; 64] = [
    HW5,HW4,HW4,RD2,RD4,RD4,RD4,RD4,  // barrier / kerb top
    HW4,HW0,HW2,HW3,RD3,RD1,RD0,RD2,  // hw lane + rd lane
    HW4,HW0,HW2,HW0,RD0,RD1,RD0,RD2,  // no dash
    HW4,HW0,HW1,HW3,RD3,RD1,RD0,RD2,  // dash
    HW4,HW0,HW1,HW3,RD3,RD1,RD0,RD2,  // dash
    HW4,HW0,HW2,HW0,RD0,RD1,RD0,RD2,  // no dash
    HW4,HW0,HW2,HW3,RD3,RD1,RD0,RD2,  // dash
    HW5,HW4,HW4,RD2,RD4,RD4,RD4,RD4,  // barrier / kerb bottom
];

// Water pipe: grass with a blue pipe marker
const WPG: u32 = 0x3068A0; // pipe outer
const WPH: u32 = 0x70B8E0; // pipe inner highlight

#[rustfmt::skip]
const WATER_PIPE: [u32; 64] = [
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,WPG,WPG,WPG,WPG,G1,G2,
    G1,G2,WPG,WPH,WPH,WPG,G2,G1,
    G2,G1,WPG,WPH,WPH,WPG,G1,G2,
    G1,G2,WPG,WPG,WPG,WPG,G2,G1,
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
];

const SU0: u32 = 0x100818; // subway very dark
const SU1: u32 = 0x2A2040; // subway dark
const SU2: u32 = 0xF0C000; // sign yellow
const SU3: u32 = 0x3C3060; // frame
const SU4: u32 = 0x5848A0; // accent blue

#[rustfmt::skip]
const SUBWAY_TUNNEL: [u32; 64] = [
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,SU0,SU1,SU1,SU0,G1,G2,
    G1,G2,SU2,SU3,SU4,SU2,G2,G1,
    G2,G1,SU2,SU4,SU3,SU2,G1,G2,
    G1,G2,SU0,SU1,SU1,SU0,G2,G1,
    G2,G1,G2,G1,G2,G1,G2,G1,
    G1,G2,G1,G2,G1,G2,G1,G2,
];

// ─── Zone tiles ───────────────────────────────────────────────────────────────
// SC2000-style corner brackets: zone colour at corners, dithered grass interior

const ZR: u32 = 0x78D840; // res border (vivid lime-green)
const ZC: u32 = 0x40A0F8; // comm border (vivid sky-blue)
const ZI: u32 = 0xF0C020; // ind border (vivid yellow)

#[rustfmt::skip]
const ZONE_RES: [u32; 64] = [
    ZR,ZR,ZR,G1,G2,ZR,ZR,ZR,
    ZR,G2,G1,G2,G1,G2,G1,ZR,
    ZR,G1,G2,G1,G2,G1,G2,ZR,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,G2,G1,G2,G1,G2,G1,
    ZR,G1,G2,G1,G2,G1,G2,ZR,
    ZR,G2,G1,G2,G1,G2,G1,ZR,
    ZR,ZR,ZR,G2,G1,ZR,ZR,ZR,
];

#[rustfmt::skip]
const ZONE_COMM: [u32; 64] = [
    ZC,ZC,ZC,G1,G2,ZC,ZC,ZC,
    ZC,G2,G1,G2,G1,G2,G1,ZC,
    ZC,G1,G2,G1,G2,G1,G2,ZC,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,G2,G1,G2,G1,G2,G1,
    ZC,G1,G2,G1,G2,G1,G2,ZC,
    ZC,G2,G1,G2,G1,G2,G1,ZC,
    ZC,ZC,ZC,G2,G1,ZC,ZC,ZC,
];

#[rustfmt::skip]
const ZONE_IND: [u32; 64] = [
    ZI,ZI,ZI,G1,G2,ZI,ZI,ZI,
    ZI,G2,G1,G2,G1,G2,G1,ZI,
    ZI,G1,G2,G1,G2,G1,G2,ZI,
    G1,G2,G1,G2,G1,G2,G1,G2,
    G2,G1,G2,G1,G2,G1,G2,G1,
    ZI,G1,G2,G1,G2,G1,G2,ZI,
    ZI,G2,G1,G2,G1,G2,G1,ZI,
    ZI,ZI,ZI,G2,G1,ZI,ZI,ZI,
];

// ─── Building helper macro ────────────────────────────────────────────────────
// Each building: BG at corners, HL=highlight strip (NW), SH=shadow strip (SE),
// then the 4×4 roof interior.
// bg = zone ground, hl = highlight, sh = shadow, a/b/c/d = 4×4 interior rows.
macro_rules! building {
    ($bg:expr, $hl:expr, $sh:expr,
     $a0:expr,$a1:expr,$a2:expr,$a3:expr,
     $b0:expr,$b1:expr,$b2:expr,$b3:expr,
     $c0:expr,$c1:expr,$c2:expr,$c3:expr,
     $d0:expr,$d1:expr,$d2:expr,$d3:expr) => {
        [
            $bg, $bg, $bg, $bg, $bg, $bg, $bg, $bg,
            $bg, $hl, $hl, $hl, $hl, $hl, $hl, $bg,
            $bg, $hl, $a0, $a1, $a2, $a3, $sh, $bg,
            $bg, $hl, $b0, $b1, $b2, $b3, $sh, $bg,
            $bg, $hl, $c0, $c1, $c2, $c3, $sh, $bg,
            $bg, $hl, $d0, $d1, $d2, $d3, $sh, $bg,
            $bg, $sh, $sh, $sh, $sh, $sh, $sh, $bg,
            $bg, $bg, $bg, $bg, $bg, $bg, $bg, $bg,
        ]
    };
}

// ─── Residential ─────────────────────────────────────────────────────────────

const R_BG: u32 = 0x1C5014; // res ground (dark forest)

// ResLow – terracotta hip roof: bright orange-red tiles, dark ridge centre
const RL_HL: u32 = 0xFFB878; // warm cream highlight
const RL_SH: u32 = 0x5C2C10; // deep shadow
const RL_RF: u32 = 0xE84820; // vivid terracotta
const RL_LT: u32 = 0xFF7030; // sun-lit tile
const RL_DK: u32 = 0xA82C10; // ridge / shadow tile

const RES_LOW: [u32; 64] = building!(
    R_BG, RL_HL, RL_SH,
    RL_LT, RL_LT, RL_RF, RL_RF,
    RL_LT, RL_DK, RL_DK, RL_RF,
    RL_RF, RL_DK, RL_DK, RL_RF,
    RL_RF, RL_RF, RL_RF, RL_RF
);

// ResMed – blue-grey flat roof with AC units
const RM_HL: u32 = 0xC8E0FF;
const RM_SH: u32 = 0x1A3050;
const RM_RF: u32 = 0x5888C0; // steel-blue roof
const RM_LT: u32 = 0x80B0E0; // sun-lit
const RM_DK: u32 = 0x304870; // shadow
const RM_AC: u32 = 0x202838; // AC unit (dark square)

const RES_MED: [u32; 64] = building!(
    R_BG, RM_HL, RM_SH,
    RM_LT, RM_LT, RM_RF, RM_RF,
    RM_LT, RM_AC, RM_RF, RM_DK,
    RM_RF, RM_RF, RM_DK, RM_DK,
    RM_RF, RM_DK, RM_DK, RM_DK
);

// ResHigh – glass tower, vivid sky-blue reflections
const RH_HL: u32 = 0xE0F8FF;
const RH_SH: u32 = 0x0C2030;
const RH_FR: u32 = 0x182838; // frame
const RH_GL: u32 = 0x40B0E8; // glass (vivid)
const RH_LT: u32 = 0xA0DCFF; // glass highlight
const RH_DK: u32 = 0x1868A0; // glass shadow

const RES_HIGH: [u32; 64] = building!(
    R_BG, RH_HL, RH_SH,
    RH_LT, RH_GL, RH_LT, RH_FR,
    RH_GL, RH_DK, RH_GL, RH_FR,
    RH_LT, RH_GL, RH_LT, RH_FR,
    RH_GL, RH_DK, RH_GL, RH_FR
);

// ─── Commercial ───────────────────────────────────────────────────────────────

const C_BG: u32 = 0x101828; // comm ground (dark navy)

// CommLow – colourful shop front: warm wall, vivid awnings
const CL_HL: u32 = 0xFFE8B0;
const CL_SH: u32 = 0x2C1808;
const CL_WL: u32 = 0xD0B878; // warm beige wall
const CL_A1: u32 = 0xE83018; // red awning
const CL_A2: u32 = 0x1870D0; // blue awning
const CL_SN: u32 = 0xF8E030; // neon sign yellow

const COMM_LOW: [u32; 64] = building!(
    C_BG, CL_HL, CL_SH,
    CL_SN, CL_SN, CL_SN, CL_SN,
    CL_A1, CL_WL, CL_WL, CL_A2,
    CL_WL, CL_A2, CL_A1, CL_WL,
    CL_A1, CL_WL, CL_WL, CL_A2
);

// CommHigh – glass office tower, vivid cyan-blue
const CH_HL: u32 = 0xD0F8FF;
const CH_SH: u32 = 0x081828;
const CH_FR: u32 = 0x182030; // dark frame
const CH_G1: u32 = 0x28B8F0; // glass vivid
const CH_G2: u32 = 0x80D8FF; // glass bright reflection
const CH_G3: u32 = 0x0870A8; // glass deep shadow

const COMM_HIGH: [u32; 64] = building!(
    C_BG, CH_HL, CH_SH,
    CH_G2, CH_G1, CH_G2, CH_FR,
    CH_G1, CH_G3, CH_G1, CH_FR,
    CH_G2, CH_G1, CH_G2, CH_FR,
    CH_FR, CH_FR, CH_FR, CH_FR
);

// ─── Industrial ───────────────────────────────────────────────────────────────

const I_BG: u32 = 0x281E08; // ind ground (very dark ochre)

// IndLight – ochre factory with skylight
const IL_HL: u32 = 0xE8D898;
const IL_SH: u32 = 0x302808;
const IL_WL: u32 = 0xB09040; // ochre wall
const IL_RF: u32 = 0x887830; // roof
const IL_SK: u32 = 0x70C0F0; // skylight (blue sky seen through)
const IL_DK: u32 = 0x504820; // shadow

const IND_LIGHT: [u32; 64] = building!(
    I_BG, IL_HL, IL_SH,
    IL_WL, IL_SK, IL_SK, IL_RF,
    IL_WL, IL_SK, IL_SK, IL_RF,
    IL_RF, IL_RF, IL_DK, IL_DK,
    IL_RF, IL_DK, IL_DK, IL_DK
);

// IndHeavy – dark machinery, heat glow, smokestacks above
const IH_HL: u32 = 0xC0B8A8;
const IH_SH: u32 = 0x100808;
const IH_WL: u32 = 0x484040; // dark wall
const IH_RF: u32 = 0x686060; // dark roof
const IH_HT: u32 = 0xFF7020; // heat glow orange
const IH_DK: u32 = 0x201818; // very dark

#[rustfmt::skip]
const IND_HEAVY: [u32; 64] = [
    I_BG, I_BG, IH_RF, IH_RF, IH_RF, IH_RF, I_BG, I_BG, // smokestacks row
    I_BG, IH_HL,IH_HL,IH_HL,IH_HL,IH_HL,IH_HL,I_BG,
    I_BG, IH_HL,IH_WL,IH_HT,IH_HT,IH_WL,IH_SH,I_BG,
    I_BG, IH_HL,IH_WL,IH_HT,IH_HT,IH_RF,IH_SH,I_BG,
    I_BG, IH_HL,IH_RF,IH_RF,IH_DK,IH_DK,IH_SH,I_BG,
    I_BG, IH_HL,IH_RF,IH_DK,IH_DK,IH_DK,IH_SH,I_BG,
    I_BG, IH_SH,IH_SH,IH_SH,IH_SH,IH_SH,IH_SH,I_BG,
    I_BG, I_BG, I_BG, I_BG, I_BG, I_BG, I_BG, I_BG,
];

// ─── Power plants ─────────────────────────────────────────────────────────────

const PP_BG: u32 = 0x0C0C18; // plant ground (near-black)

const PC_HL: u32 = 0xA0A0C0;
const PC_SH: u32 = 0x040408;
const PC_WL: u32 = 0x202028; // very dark wall
const PC_RF: u32 = 0x383848;
const PC_GL: u32 = 0xFF9020; // furnace glow
const PC_SM: u32 = 0xC0C8D0; // smoke plume

#[rustfmt::skip]
const COAL_PLANT: [u32; 64] = [
    PP_BG,PC_SM,PP_BG,PC_SM,PC_SM,PP_BG,PC_SM,PP_BG, // smoke above
    PP_BG,PC_HL,PC_HL,PC_HL,PC_HL,PC_HL,PC_HL,PP_BG,
    PP_BG,PC_HL,PC_WL,PC_WL,PC_WL,PC_WL,PC_SH,PP_BG,
    PP_BG,PC_HL,PC_WL,PC_GL,PC_GL,PC_RF,PC_SH,PP_BG,
    PP_BG,PC_HL,PC_WL,PC_GL,PC_GL,PC_RF,PC_SH,PP_BG,
    PP_BG,PC_HL,PC_RF,PC_RF,PC_RF,PC_RF,PC_SH,PP_BG,
    PP_BG,PC_SH,PC_SH,PC_SH,PC_SH,PC_SH,PC_SH,PP_BG,
    PP_BG,PP_BG,PP_BG,PP_BG,PP_BG,PP_BG,PP_BG,PP_BG,
];

const PG_HL: u32 = 0xB0C0D8;
const PG_SH: u32 = 0x081020;
const PG_TK: u32 = 0x4870A0; // gas tank blue-grey
const PG_TL: u32 = 0x90B8D8; // tank highlight
const PG_PI: u32 = 0xD0C8A0; // pipe metallic
const PG_WL: u32 = 0x283848;

const GAS_PLANT: [u32; 64] = building!(
    PP_BG, PG_HL, PG_SH,
    PG_TL, PG_TK, PG_TL, PG_WL,
    PG_TK, PG_TL, PG_TK, PG_WL,
    PG_PI, PG_PI, PG_PI, PG_PI,
    PG_WL, PG_WL, PG_WL, PG_WL
);

// ─── Park ─────────────────────────────────────────────────────────────────────

const PK1: u32 = 0x3A9028; // park base
const PK2: u32 = 0x52B838; // park bright
const PK3: u32 = 0x70D050; // vivid highlight
const PK4: u32 = 0xD4B870; // sand path
const PK5: u32 = 0xE8D090; // path light
const PK7: u32 = 0xE04060; // flowers red
const PK8: u32 = 0xF8E030; // flowers yellow

#[rustfmt::skip]
const PARK: [u32; 64] = [
    PK3,PK2,PK1,PK4,PK5,PK4,PK2,PK3,
    PK2,PK7,PK2,PK4,PK4,PK2,PK8,PK2,
    PK1,PK2,PK3,PK5,PK4,PK3,PK2,PK1,
    PK4,PK4,PK5,PK4,PK4,PK5,PK4,PK4,
    PK5,PK4,PK4,PK5,PK5,PK4,PK4,PK5,
    PK2,PK1,PK3,PK4,PK5,PK3,PK1,PK2,
    PK2,PK8,PK2,PK4,PK4,PK2,PK7,PK2,
    PK3,PK2,PK1,PK5,PK4,PK1,PK2,PK3,
];

// ─── Services ─────────────────────────────────────────────────────────────────

const SV_BG: u32 = 0x101820; // service building ground

// Police – deep blue with gold badge
const PO_HL: u32 = 0xC0D8FF;
const PO_SH: u32 = 0x040818;
const PO_WL: u32 = 0x1038B0; // vivid police blue
const PO_RF: u32 = 0x1848D8;
const PO_BD: u32 = 0xF8D800; // gold badge
const PO_WH: u32 = 0xF0F0FF; // white trim

const POLICE: [u32; 64] = building!(
    SV_BG, PO_HL, PO_SH,
    PO_RF, PO_RF, PO_RF, PO_RF,
    PO_BD, PO_WH, PO_WH, PO_WL,
    PO_WH, PO_WH, PO_WL, PO_WL,
    PO_WL, PO_WL, PO_WL, PO_WL
);

// Fire dept – vivid red with orange flash
const FI_HL: u32 = 0xFFD0B0;
const FI_SH: u32 = 0x300008;
const FI_WL: u32 = 0xC01020; // fire-engine red
const FI_RF: u32 = 0xE02030;
const FI_OR: u32 = 0xFF8020; // orange emergency light
const FI_WH: u32 = 0xFFF0F0; // white trim

const FIRE_DEPT: [u32; 64] = building!(
    SV_BG, FI_HL, FI_SH,
    FI_OR, FI_OR, FI_RF, FI_RF,
    FI_WH, FI_WH, FI_WL, FI_WL,
    FI_RF, FI_WL, FI_WL, FI_WL,
    FI_WL, FI_WL, FI_WL, FI_WL
);

// Hospital – clean white with red cross
const HO_HL: u32 = 0xFFFFFF;
const HO_SH: u32 = 0x282828;
const HO_WH: u32 = 0xF0F0F0; // white
const HO_GR: u32 = 0xD0D0D0; // light grey
const HO_CR: u32 = 0xE01010; // red cross
const HO_BL: u32 = 0x80C0E8; // blue window

const HOSPITAL: [u32; 64] = building!(
    SV_BG, HO_HL, HO_SH,
    HO_WH, HO_CR, HO_CR, HO_GR,
    HO_CR, HO_CR, HO_CR, HO_GR,
    HO_WH, HO_CR, HO_CR, HO_GR,
    HO_BL, HO_WH, HO_WH, HO_GR
);

// Bus depot – warm ochre with vivid yellow bus
const BD_HL: u32 = 0xFFE8A0;
const BD_SH: u32 = 0x281808;
const BD_WL: u32 = 0x906840; // depot wall
const BD_YL: u32 = 0xF8C000; // bus yellow
const BD_BL: u32 = 0x90C8F0; // bus window
const BD_DK: u32 = 0x302010; // dark

const BUS_DEPOT: [u32; 64] = building!(
    SV_BG, BD_HL, BD_SH,
    BD_WL, BD_WL, BD_WL, BD_WL,
    BD_YL, BD_BL, BD_BL, BD_YL,
    BD_YL, BD_BL, BD_BL, BD_YL,
    BD_DK, BD_YL, BD_YL, BD_DK
);

// Rail depot – silver-grey station with platform
const RD2_HL: u32 = 0xE0E8F0;
const RD2_SH: u32 = 0x101820;
const RD2_WL: u32 = 0x607080; // steel wall
const RD2_RF: u32 = 0x809090; // roof
const RD2_PL: u32 = 0xC8C0A8; // platform tan
const RD2_TR: u32 = 0xD8D8E8; // track steel

const RAIL_DEPOT: [u32; 64] = building!(
    SV_BG, RD2_HL, RD2_SH,
    RD2_TR, RD2_TR, RD2_TR, RD2_TR,
    RD2_WL, RD2_PL, RD2_PL, RD2_WL,
    RD2_WL, RD2_RF, RD2_RF, RD2_WL,
    RD2_TR, RD2_TR, RD2_TR, RD2_TR
);

// Subway station – dark with glowing yellow sign
const SS_HL: u32 = 0x9090C0;
const SS_SH: u32 = 0x080010;
const SS_WL: u32 = 0x2C2848; // dark purple-grey
const SS_SN: u32 = 0xF8D800; // sign yellow
const SS_BL: u32 = 0x5040A0; // accent blue
const SS_DK: u32 = 0x100818; // very dark

const SUBWAY_STATION: [u32; 64] = building!(
    SV_BG, SS_HL, SS_SH,
    SS_SN, SS_SN, SS_WL, SS_WL,
    SS_BL, SS_DK, SS_DK, SS_WL,
    SS_BL, SS_DK, SS_DK, SS_WL,
    SS_WL, SS_WL, SS_WL, SS_WL
);

// ─── Water infrastructure ─────────────────────────────────────────────────────

const WI_BG: u32 = 0x101C28; // water infra ground

// Water pump – vivid blue machinery
const WP_HL: u32 = 0xC0E8FF;
const WP_SH: u32 = 0x041018;
const WP_MH: u32 = 0x2878C8; // machinery dark
const WP_ML: u32 = 0x60B8E8; // machinery light
const WP_SH2: u32 = 0x80D8F8; // shiny highlight
const WP_DK: u32 = 0x102840; // dark

const WATER_PUMP: [u32; 64] = building!(
    WI_BG, WP_HL, WP_SH,
    WP_ML, WP_SH2, WP_ML, WP_DK,
    WP_SH2, WP_MH, WP_SH2, WP_DK,
    WP_ML, WP_SH2, WP_ML, WP_DK,
    WP_DK, WP_DK, WP_DK, WP_DK
);

// Water tower – round tank on legs, vivid blue
const WT_SH: u32 = 0x082030;
const WT_TK: u32 = 0x3888C8; // tank blue
const WT_TL: u32 = 0x90C8F0; // tank highlight
const WT_LG: u32 = 0x486070; // leg metal
const WT_WH: u32 = 0xC8E8FF; // water surface

#[rustfmt::skip]
const WATER_TOWER: [u32; 64] = [
    G1,G2,WT_TL,WT_WH,WT_WH,WT_TL,G2,G1,
    G2,WT_TK,WT_TL,WT_WH,WT_WH,WT_TL,WT_TK,G2,
    G1,WT_TK,WT_TK,WT_TK,WT_TK,WT_TK,WT_SH,G1,
    G2,WT_TK,WT_TK,WT_TK,WT_TK,WT_SH,WT_SH,G2,
    G1,G2,WT_LG,G1,G2,WT_LG,G2,G1,
    G2,G1,WT_LG,G2,G1,WT_LG,G1,G2,
    G1,WT_LG,G2,G1,G2,G1,WT_LG,G1,
    G2,G1,G2,G1,G2,G1,G2,G2,
];

// Water treatment – blue treatment pools
const WTR_HL: u32 = 0xC0D8F0;
const WTR_SH: u32 = 0x081828;
const WTR_WL: u32 = 0x3870A0; // wall
const WTR_PL: u32 = 0x1858C0; // pool deep
const WTR_FL: u32 = 0x68C0E8; // pool foam
const WTR_RF: u32 = 0x507090; // roof

const WATER_TREATMENT: [u32; 64] = building!(
    WI_BG, WTR_HL, WTR_SH,
    WTR_PL, WTR_FL, WTR_FL, WTR_WL,
    WTR_FL, WTR_PL, WTR_FL, WTR_WL,
    WTR_FL, WTR_FL, WTR_PL, WTR_WL,
    WTR_RF, WTR_RF, WTR_RF, WTR_RF
);

// Desalination – ocean blue with pipe vents
const DS_HL: u32 = 0xB0D8F8;
const DS_SH: u32 = 0x041830;
const DS_WL: u32 = 0x1A5898;
const DS_OC: u32 = 0x0870D0; // ocean intake vivid
const DS_PI: u32 = 0x8098C0; // pipe
const DS_SH2: u32 = 0x60A8E8; // sheen

const DESALINATION: [u32; 64] = building!(
    WI_BG, DS_HL, DS_SH,
    DS_OC, DS_SH2, DS_OC, DS_WL,
    DS_SH2, DS_PI, DS_PI, DS_WL,
    DS_OC, DS_PI, DS_OC, DS_WL,
    DS_WL, DS_WL, DS_WL, DS_WL
);

// ─── Rubble ───────────────────────────────────────────────────────────────────

const RB0: u32 = 0x302818;
const RB1: u32 = 0x504030;
const RB2: u32 = 0x706050;
const RB3: u32 = 0x504848; // concrete chunk
const RB4: u32 = 0x706868; // concrete light

#[rustfmt::skip]
const RUBBLE: [u32; 64] = [
    RB0,RB3,RB1,RB4,RB0,RB3,RB2,RB1,
    RB3,RB1,RB4,RB2,RB3,RB1,RB0,RB3,
    RB2,RB4,RB1,RB0,RB4,RB2,RB3,RB2,
    RB1,RB2,RB3,RB4,RB1,RB4,RB1,RB4,
    RB4,RB0,RB4,RB1,RB2,RB0,RB4,RB2,
    RB3,RB2,RB1,RB3,RB4,RB3,RB2,RB1,
    RB1,RB4,RB2,RB1,RB3,RB1,RB4,RB0,
    RB0,RB1,RB3,RB4,RB0,RB2,RB1,RB3,
];
