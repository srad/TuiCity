use font8x8::UnicodeFonts;
use ratatui::style::Color;

/// Compute a display scale from the window's DPI scale factor.
/// Base scale is 1, so tiles are 8×8 physical pixels at 1× DPI —
/// giving more detail on screen. On HiDPI (2×) it becomes 16×16.
pub fn cell_scale(dpi_scale: f64) -> u32 {
    dpi_scale.ceil().max(1.0) as u32
}

/// Width of one character cell at the given scale (= 8 * scale pixels).
pub fn cell_w(scale: u32) -> u32 {
    8 * scale
}

/// Height of one character cell at the given scale (= 8 * scale pixels).
pub fn cell_h(scale: u32) -> u32 {
    8 * scale
}

/// Convert a ratatui `Color` to `0x00RRGGBB` (softbuffer format).
pub fn color_to_u32(color: Color) -> u32 {
    match color {
        Color::Rgb(r, g, b) => rgb_to_u32(r, g, b),
        _ => 0x000000,
    }
}

pub fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn get_glyph(ch: char) -> [u8; 8] {
    font8x8::BASIC_FONTS
        .get(ch)
        .or_else(|| font8x8::BLOCK_FONTS.get(ch))
        .or_else(|| font8x8::BOX_FONTS.get(ch))
        .unwrap_or([0u8; 8])
}

/// Draw a single character at pixel position (px, py).
/// Each font pixel is rendered as a `scale × scale` block.
/// `font8x8` stores bit 0 as the leftmost pixel of each row.
pub fn draw_char(
    buf: &mut [u32],
    buf_width: u32,
    px: u32,
    py: u32,
    ch: char,
    fg: u32,
    bg: u32,
    scale: u32,
) {
    let glyph = get_glyph(ch);
    for row in 0..8u32 {
        let byte = glyph[row as usize];
        for col in 0..8u32 {
            // Bit 0 = leftmost pixel (font8x8 convention)
            let set = (byte >> col) & 1 != 0;
            let color = if set { fg } else { bg };
            for sy in 0..scale {
                for sx in 0..scale {
                    let x = px + col * scale + sx;
                    let y = py + row * scale + sy;
                    let idx = (y * buf_width + x) as usize;
                    if let Some(pixel) = buf.get_mut(idx) {
                        *pixel = color;
                    }
                }
            }
        }
    }
}

/// Draw a string starting at pixel position (px, py).
pub fn draw_str(
    buf: &mut [u32],
    buf_width: u32,
    px: u32,
    py: u32,
    s: &str,
    fg: u32,
    bg: u32,
    scale: u32,
) {
    let cw = cell_w(scale);
    for (i, ch) in s.chars().enumerate() {
        draw_char(buf, buf_width, px + i as u32 * cw, py, ch, fg, bg, scale);
    }
}

/// Fill a rectangle with a solid color.
pub fn fill_rect(buf: &mut [u32], buf_width: u32, x: u32, y: u32, w: u32, h: u32, color: u32) {
    for row in y..y + h {
        let start = (row * buf_width + x) as usize;
        let end = (start + w as usize).min(buf.len());
        if start < buf.len() {
            buf[start..end].fill(color);
        }
    }
}

/// Draw a 1-pixel-wide horizontal line.
pub fn hline(buf: &mut [u32], buf_width: u32, x: u32, y: u32, w: u32, color: u32) {
    fill_rect(buf, buf_width, x, y, w, 1, color);
}

/// Draw a 1-pixel-wide vertical line.
pub fn vline(buf: &mut [u32], buf_width: u32, x: u32, y: u32, h: u32, color: u32) {
    fill_rect(buf, buf_width, x, y, 1, h, color);
}

/// Draw a rectangle outline (1 pixel thick).
pub fn draw_rect_outline(
    buf: &mut [u32],
    buf_width: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: u32,
) {
    if w == 0 || h == 0 {
        return;
    }
    hline(buf, buf_width, x, y, w, color);
    hline(buf, buf_width, x, y + h - 1, w, color);
    vline(buf, buf_width, x, y, h, color);
    vline(buf, buf_width, x + w - 1, y, h, color);
}
