use crate::core::{
    map::{Tile, TileOverlay},
    sim::TaxSector,
};
use ratatui::style::Color;
use std::sync::{OnceLock, RwLock};

// ── Feature 1: Overlay mode ───────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum OverlayMode {
    #[default]
    None,
    Power,
    Water,
    Traffic,
    Pollution,
    LandValue,
    Crime,
    FireRisk,
}

impl OverlayMode {
    pub fn next(self) -> Self {
        match self {
            OverlayMode::None => OverlayMode::Power,
            OverlayMode::Power => OverlayMode::Water,
            OverlayMode::Water => OverlayMode::Traffic,
            OverlayMode::Traffic => OverlayMode::Pollution,
            OverlayMode::Pollution => OverlayMode::LandValue,
            OverlayMode::LandValue => OverlayMode::Crime,
            OverlayMode::Crime => OverlayMode::FireRisk,
            OverlayMode::FireRisk => OverlayMode::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OverlayMode::None => "",
            OverlayMode::Power => "[Overlay: Power Grid]",
            OverlayMode::Water => "[Overlay: Water Service]",
            OverlayMode::Traffic => "[Overlay: Traffic]",
            OverlayMode::Pollution => "[Overlay: Pollution]",
            OverlayMode::LandValue => "[Overlay: Land Value]",
            OverlayMode::Crime => "[Overlay: Crime Rate]",
            OverlayMode::FireRisk => "[Overlay: Fire Risk]",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    Copper,
    Harbor,
    Sunset,
    Emerald,
    Candy,
    Metro,
    Violet,
}

pub const ALL_THEME_PRESETS: [ThemePreset; 7] = [
    ThemePreset::Copper,
    ThemePreset::Harbor,
    ThemePreset::Sunset,
    ThemePreset::Emerald,
    ThemePreset::Candy,
    ThemePreset::Metro,
    ThemePreset::Violet,
];

impl ThemePreset {
    pub fn label(self) -> &'static str {
        match self {
            ThemePreset::Copper => "Copper Classic",
            ThemePreset::Harbor => "Harbor Blue",
            ThemePreset::Sunset => "Sunset Pop",
            ThemePreset::Emerald => "Emerald Night",
            ThemePreset::Candy => "Candy Arcade",
            ThemePreset::Metro => "Metro Mint",
            ThemePreset::Violet => "Violet Pulse",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ThemePreset::Copper => ThemePreset::Harbor,
            ThemePreset::Harbor => ThemePreset::Sunset,
            ThemePreset::Sunset => ThemePreset::Emerald,
            ThemePreset::Emerald => ThemePreset::Candy,
            ThemePreset::Candy => ThemePreset::Metro,
            ThemePreset::Metro => ThemePreset::Violet,
            ThemePreset::Violet => ThemePreset::Copper,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderStyle {
    TerminalAscii,
    PixelDos,
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
    pub window_shadow: Color,
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
    pub news_ticker_bg: Color,
    pub news_ticker_label_bg: Color,
    pub news_ticker_label_fg: Color,
    pub news_ticker_text: Color,
    pub news_ticker_alert: Color,
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

fn active_theme_lock() -> &'static RwLock<ThemePreset> {
    static ACTIVE_THEME: OnceLock<RwLock<ThemePreset>> = OnceLock::new();
    ACTIVE_THEME.get_or_init(|| RwLock::new(ThemePreset::Copper))
}

fn render_style_lock() -> &'static RwLock<RenderStyle> {
    static RENDER_STYLE: OnceLock<RwLock<RenderStyle>> = OnceLock::new();
    RENDER_STYLE.get_or_init(|| RwLock::new(RenderStyle::TerminalAscii))
}

pub fn current_theme() -> ThemePreset {
    *active_theme_lock()
        .read()
        .expect("theme lock should not be poisoned")
}

pub fn set_theme(theme: ThemePreset) {
    *active_theme_lock()
        .write()
        .expect("theme lock should not be poisoned") = theme;
}

pub fn cycle_theme() -> ThemePreset {
    let mut guard = active_theme_lock()
        .write()
        .expect("theme lock should not be poisoned");
    *guard = guard.next();
    *guard
}

pub fn set_render_style(style: RenderStyle) {
    *render_style_lock()
        .write()
        .expect("render style lock should not be poisoned") = style;
}

pub fn render_style() -> RenderStyle {
    *render_style_lock()
        .read()
        .expect("render style lock should not be poisoned")
}

pub fn is_pixel_style() -> bool {
    matches!(render_style(), RenderStyle::PixelDos)
}

pub fn ui_palette() -> UiPalette {
    let palette = palette_for(current_theme());
    if is_pixel_style() {
        pixel_palette(palette)
    } else {
        palette
    }
}

fn pixel_palette(mut ui: UiPalette) -> UiPalette {
    ui.desktop_bg = Color::Rgb(0, 0, 40);
    ui.title = Color::Rgb(255, 255, 85);
    ui.subtitle = Color::Rgb(170, 255, 255);
    ui.window_bg = Color::Rgb(0, 0, 96);
    ui.window_border = Color::Rgb(85, 255, 255);
    ui.window_title = Color::Rgb(255, 255, 170);
    ui.window_shadow = Color::Rgb(0, 0, 20);
    ui.map_window_bg = Color::Rgb(0, 0, 32);
    ui.panel_window_bg = Color::Rgb(0, 0, 96);
    ui.budget_window_bg = Color::Rgb(0, 0, 96);
    ui.inspect_window_bg = Color::Rgb(0, 0, 72);
    ui.popup_bg = Color::Rgb(0, 0, 112);
    ui.popup_border = Color::Rgb(170, 170, 170);
    ui.popup_title = Color::Rgb(255, 255, 85);
    ui.text_primary = Color::Rgb(170, 170, 170);
    ui.text_secondary = Color::Rgb(170, 255, 255);
    ui.text_muted = Color::Rgb(120, 180, 180);
    ui.text_dim = Color::Rgb(90, 110, 140);
    ui.accent = Color::Rgb(85, 255, 255);
    ui.accent_soft = Color::Rgb(85, 85, 255);
    ui.selection_bg = Color::Rgb(170, 170, 170);
    ui.selection_fg = Color::Rgb(0, 0, 96);
    ui.success = Color::Rgb(85, 255, 85);
    ui.danger = Color::Rgb(255, 85, 85);
    ui.warning = Color::Rgb(255, 255, 85);
    ui.info = Color::Rgb(85, 255, 255);
    ui.menu_bg = Color::Rgb(0, 0, 96);
    ui.menu_fg = Color::Rgb(170, 170, 170);
    ui.menu_focus_bg = Color::Rgb(170, 170, 170);
    ui.menu_focus_fg = Color::Rgb(0, 0, 96);
    ui.menu_hotkey = Color::Rgb(255, 255, 85);
    ui.menu_right = Color::Rgb(170, 255, 255);
    ui.menu_title = Color::Rgb(255, 255, 170);
    ui.toolbar_bg = Color::Rgb(0, 0, 96);
    ui.toolbar_header = Color::Rgb(255, 255, 85);
    ui.toolbar_rule = Color::Rgb(85, 255, 255);
    ui.toolbar_button_bg = Color::Rgb(0, 0, 128);
    ui.toolbar_button_fg = Color::Rgb(170, 170, 170);
    ui.toolbar_active_bg = Color::Rgb(170, 170, 170);
    ui.toolbar_active_fg = Color::Rgb(0, 0, 96);
    ui.toolbar_armed_bg = Color::Rgb(255, 255, 85);
    ui.status_bg = Color::Rgb(0, 0, 96);
    ui.status_sep = Color::Rgb(85, 255, 255);
    ui.status_city = Color::Rgb(255, 255, 170);
    ui.status_population = Color::Rgb(170, 170, 170);
    ui.status_date = Color::Rgb(170, 255, 255);
    ui.status_message = Color::Rgb(255, 255, 85);
    ui.status_button_run_bg = Color::Rgb(85, 255, 85);
    ui.status_button_run_fg = Color::Rgb(0, 0, 32);
    ui.status_button_pause_bg = Color::Rgb(170, 170, 170);
    ui.status_button_pause_fg = Color::Rgb(0, 0, 96);
    ui.news_ticker_bg = Color::Rgb(0, 0, 48);
    ui.news_ticker_label_bg = Color::Rgb(170, 170, 170);
    ui.news_ticker_label_fg = Color::Rgb(0, 0, 96);
    ui.news_ticker_text = Color::Rgb(170, 255, 255);
    ui.news_ticker_alert = Color::Rgb(255, 255, 85);
    ui.input_bg = Color::Rgb(0, 0, 128);
    ui.input_fg = Color::Rgb(170, 170, 170);
    ui.input_focus_bg = Color::Rgb(170, 170, 170);
    ui.input_focus_fg = Color::Rgb(0, 0, 96);
    ui.button_bg = Color::Rgb(0, 0, 128);
    ui.button_fg = Color::Rgb(170, 170, 170);
    ui.button_focus_bg = Color::Rgb(170, 170, 170);
    ui.button_focus_fg = Color::Rgb(0, 0, 96);
    ui.button_armed_bg = Color::Rgb(255, 255, 85);
    ui.scrollbar_button_fg = Color::Rgb(255, 255, 85);
    ui.scrollbar_button_bg = Color::Rgb(0, 0, 96);
    ui.scrollbar_track_fg = Color::Rgb(85, 255, 255);
    ui.scrollbar_track_bg = Color::Rgb(0, 0, 96);
    ui.scrollbar_thumb_fg = Color::Rgb(170, 170, 170);
    ui.scrollbar_thumb_bg = Color::Rgb(0, 0, 32);
    ui.viewport_outline = Color::Rgb(170, 255, 255);
    ui.disaster_bg = Color::Rgb(96, 0, 0);
    ui.disaster_border = Color::Rgb(255, 255, 85);
    ui.disaster_select_bg = Color::Rgb(170, 170, 170);
    ui
}

pub fn palette_for(theme: ThemePreset) -> UiPalette {
    let base = copper_palette();
    let palette = match theme {
        ThemePreset::Copper => base,
        ThemePreset::Harbor => UiPalette {
            desktop_bg: Color::Rgb(12, 22, 34),
            title: Color::Rgb(131, 221, 246),
            subtitle: Color::Rgb(182, 214, 227),
            window_bg: Color::Rgb(20, 34, 46),
            window_border: Color::Rgb(113, 176, 190),
            window_title: Color::Rgb(216, 241, 247),
            window_shadow: Color::Rgb(7, 12, 19),
            map_window_bg: Color::Rgb(14, 24, 30),
            panel_window_bg: Color::Rgb(19, 33, 43),
            budget_window_bg: Color::Rgb(17, 29, 39),
            inspect_window_bg: Color::Rgb(16, 27, 35),
            popup_bg: Color::Rgb(25, 39, 50),
            popup_border: Color::Rgb(112, 195, 204),
            popup_title: Color::Rgb(181, 242, 243),
            text_primary: Color::Rgb(224, 240, 241),
            text_secondary: Color::Rgb(180, 205, 210),
            text_muted: Color::Rgb(129, 157, 164),
            text_dim: Color::Rgb(91, 118, 126),
            accent: Color::Rgb(109, 213, 221),
            accent_soft: Color::Rgb(130, 187, 220),
            selection_bg: Color::Rgb(63, 124, 150),
            selection_fg: Color::Rgb(238, 249, 250),
            success: Color::Rgb(125, 216, 168),
            danger: Color::Rgb(239, 127, 120),
            warning: Color::Rgb(243, 198, 111),
            info: Color::Rgb(129, 206, 241),
            sector_residential: Color::Rgb(131, 234, 155),
            sector_residential_bg: Color::Rgb(18, 73, 57),
            sector_commercial: Color::Rgb(126, 210, 255),
            sector_commercial_bg: Color::Rgb(20, 64, 100),
            sector_industrial: Color::Rgb(250, 201, 117),
            sector_industrial_bg: Color::Rgb(91, 69, 24),
            menu_bg: Color::Rgb(170, 213, 223),
            menu_fg: Color::Rgb(18, 34, 41),
            menu_focus_bg: Color::Rgb(57, 105, 129),
            menu_focus_fg: Color::Rgb(236, 248, 250),
            menu_hotkey: Color::Rgb(63, 103, 124),
            menu_right: Color::Rgb(57, 84, 97),
            menu_title: Color::Rgb(27, 61, 75),
            toolbar_bg: Color::Rgb(16, 30, 40),
            toolbar_header: Color::Rgb(178, 228, 231),
            toolbar_rule: Color::Rgb(83, 115, 126),
            toolbar_button_bg: Color::Rgb(40, 57, 69),
            toolbar_button_fg: Color::Rgb(224, 239, 241),
            toolbar_active_bg: Color::Rgb(105, 209, 220),
            toolbar_active_fg: Color::Rgb(15, 28, 35),
            toolbar_armed_bg: Color::Rgb(75, 147, 182),
            status_bg: Color::Rgb(18, 31, 41),
            status_sep: Color::Rgb(83, 112, 121),
            status_city: Color::Rgb(133, 229, 243),
            status_population: Color::Rgb(171, 233, 202),
            status_date: Color::Rgb(215, 235, 238),
            status_message: Color::Rgb(255, 214, 135),
            status_button_run_bg: Color::Rgb(69, 146, 117),
            status_button_run_fg: Color::Rgb(236, 247, 244),
            status_button_pause_bg: Color::Rgb(84, 121, 146),
            status_button_pause_fg: Color::Rgb(224, 241, 248),
            news_ticker_bg: Color::Rgb(13, 25, 34),
            news_ticker_label_bg: Color::Rgb(57, 105, 129),
            news_ticker_label_fg: Color::Rgb(236, 248, 250),
            news_ticker_text: Color::Rgb(230, 241, 244),
            news_ticker_alert: Color::Rgb(255, 205, 124),
            input_bg: Color::Rgb(43, 60, 71),
            input_fg: Color::Rgb(236, 244, 245),
            input_focus_bg: Color::Rgb(112, 214, 223),
            input_focus_fg: Color::Rgb(15, 29, 35),
            slider_bg: Color::Rgb(58, 81, 93),
            slider_water_fg: Color::Rgb(111, 201, 235),
            slider_water_focus_bg: Color::Rgb(84, 101, 115),
            slider_trees_fg: Color::Rgb(120, 213, 152),
            slider_trees_focus_bg: Color::Rgb(77, 97, 100),
            button_bg: Color::Rgb(29, 49, 60),
            button_fg: Color::Rgb(221, 237, 239),
            button_focus_bg: Color::Rgb(109, 213, 221),
            button_focus_fg: Color::Rgb(15, 28, 35),
            button_armed_bg: Color::Rgb(74, 145, 173),
            scrollbar_button_fg: Color::Rgb(17, 31, 39),
            scrollbar_button_bg: Color::Rgb(176, 214, 223),
            scrollbar_track_fg: Color::Rgb(72, 99, 108),
            scrollbar_track_bg: Color::Rgb(31, 46, 56),
            scrollbar_thumb_fg: Color::Rgb(236, 246, 247),
            scrollbar_thumb_bg: Color::Rgb(83, 140, 157),
            scrollbar_corner_fg: Color::Rgb(72, 99, 108),
            scrollbar_corner_bg: Color::Rgb(176, 214, 223),
            preview_valid_fg: Color::Rgb(135, 239, 194),
            preview_valid_bg: Color::Rgb(21, 93, 72),
            preview_invalid_fg: Color::Rgb(255, 161, 154),
            preview_invalid_bg: Color::Rgb(101, 35, 35),
            preview_line_fg: Color::Rgb(151, 227, 246),
            preview_line_bg: Color::Rgb(26, 76, 91),
            viewport_outline: Color::Rgb(180, 237, 241),
            disaster_bg: Color::Rgb(60, 26, 31),
            disaster_border: Color::Rgb(222, 109, 102),
            disaster_select_bg: Color::Rgb(109, 48, 52),
        },
        ThemePreset::Sunset => UiPalette {
            desktop_bg: Color::Rgb(44, 20, 37),
            title: Color::Rgb(255, 207, 106),
            subtitle: Color::Rgb(255, 168, 142),
            window_bg: Color::Rgb(60, 28, 50),
            window_border: Color::Rgb(255, 132, 145),
            window_title: Color::Rgb(255, 228, 173),
            window_shadow: Color::Rgb(20, 8, 16),
            map_window_bg: Color::Rgb(28, 19, 31),
            panel_window_bg: Color::Rgb(55, 25, 45),
            budget_window_bg: Color::Rgb(50, 22, 41),
            inspect_window_bg: Color::Rgb(43, 22, 37),
            popup_bg: Color::Rgb(67, 31, 54),
            popup_border: Color::Rgb(255, 145, 121),
            popup_title: Color::Rgb(255, 218, 129),
            text_primary: Color::Rgb(255, 232, 220),
            text_secondary: Color::Rgb(242, 190, 177),
            text_muted: Color::Rgb(200, 138, 138),
            text_dim: Color::Rgb(140, 95, 98),
            accent: Color::Rgb(255, 173, 100),
            accent_soft: Color::Rgb(255, 130, 180),
            selection_bg: Color::Rgb(255, 204, 107),
            selection_fg: Color::Rgb(63, 23, 35),
            success: Color::Rgb(162, 226, 129),
            danger: Color::Rgb(255, 118, 111),
            warning: Color::Rgb(255, 194, 97),
            info: Color::Rgb(255, 143, 187),
            sector_residential: Color::Rgb(156, 239, 132),
            sector_residential_bg: Color::Rgb(40, 88, 30),
            sector_commercial: Color::Rgb(122, 218, 255),
            sector_commercial_bg: Color::Rgb(23, 67, 111),
            sector_industrial: Color::Rgb(255, 201, 109),
            sector_industrial_bg: Color::Rgb(98, 68, 20),
            menu_bg: Color::Rgb(255, 205, 164),
            menu_fg: Color::Rgb(68, 26, 38),
            menu_focus_bg: Color::Rgb(255, 134, 121),
            menu_focus_fg: Color::Rgb(255, 245, 235),
            menu_hotkey: Color::Rgb(141, 68, 64),
            menu_right: Color::Rgb(128, 73, 63),
            menu_title: Color::Rgb(116, 41, 59),
            toolbar_bg: Color::Rgb(53, 24, 43),
            toolbar_header: Color::Rgb(255, 223, 168),
            toolbar_rule: Color::Rgb(144, 83, 87),
            toolbar_button_bg: Color::Rgb(78, 39, 63),
            toolbar_button_fg: Color::Rgb(255, 232, 220),
            toolbar_active_bg: Color::Rgb(255, 204, 107),
            toolbar_active_fg: Color::Rgb(67, 27, 39),
            toolbar_armed_bg: Color::Rgb(255, 126, 145),
            status_bg: Color::Rgb(58, 25, 43),
            status_sep: Color::Rgb(139, 82, 84),
            status_city: Color::Rgb(255, 214, 112),
            status_population: Color::Rgb(255, 165, 180),
            status_date: Color::Rgb(255, 226, 211),
            status_message: Color::Rgb(255, 194, 97),
            status_button_run_bg: Color::Rgb(103, 151, 69),
            status_button_run_fg: Color::Rgb(247, 245, 230),
            status_button_pause_bg: Color::Rgb(161, 87, 73),
            status_button_pause_fg: Color::Rgb(255, 232, 183),
            news_ticker_bg: Color::Rgb(45, 18, 34),
            news_ticker_label_bg: Color::Rgb(255, 134, 121),
            news_ticker_label_fg: Color::Rgb(255, 245, 235),
            news_ticker_text: Color::Rgb(255, 232, 220),
            news_ticker_alert: Color::Rgb(255, 204, 107),
            input_bg: Color::Rgb(88, 44, 70),
            input_fg: Color::Rgb(255, 237, 228),
            input_focus_bg: Color::Rgb(255, 204, 107),
            input_focus_fg: Color::Rgb(67, 27, 39),
            slider_bg: Color::Rgb(111, 65, 83),
            slider_water_fg: Color::Rgb(118, 219, 255),
            slider_water_focus_bg: Color::Rgb(132, 83, 92),
            slider_trees_fg: Color::Rgb(157, 225, 117),
            slider_trees_focus_bg: Color::Rgb(120, 88, 74),
            button_bg: Color::Rgb(70, 33, 56),
            button_fg: Color::Rgb(255, 232, 220),
            button_focus_bg: Color::Rgb(255, 204, 107),
            button_focus_fg: Color::Rgb(67, 27, 39),
            button_armed_bg: Color::Rgb(255, 129, 144),
            scrollbar_button_fg: Color::Rgb(61, 24, 33),
            scrollbar_button_bg: Color::Rgb(255, 205, 164),
            scrollbar_track_fg: Color::Rgb(126, 75, 76),
            scrollbar_track_bg: Color::Rgb(64, 31, 50),
            scrollbar_thumb_fg: Color::Rgb(255, 242, 231),
            scrollbar_thumb_bg: Color::Rgb(179, 99, 111),
            scrollbar_corner_fg: Color::Rgb(126, 75, 76),
            scrollbar_corner_bg: Color::Rgb(255, 205, 164),
            preview_valid_fg: Color::Rgb(159, 244, 183),
            preview_valid_bg: Color::Rgb(34, 98, 62),
            preview_invalid_fg: Color::Rgb(255, 169, 162),
            preview_invalid_bg: Color::Rgb(109, 31, 41),
            preview_line_fg: Color::Rgb(168, 235, 255),
            preview_line_bg: Color::Rgb(58, 89, 110),
            viewport_outline: Color::Rgb(255, 214, 112),
            disaster_bg: Color::Rgb(69, 19, 28),
            disaster_border: Color::Rgb(255, 112, 105),
            disaster_select_bg: Color::Rgb(126, 42, 54),
        },
        ThemePreset::Emerald => UiPalette {
            desktop_bg: Color::Rgb(10, 26, 22),
            title: Color::Rgb(150, 244, 185),
            subtitle: Color::Rgb(136, 216, 202),
            window_bg: Color::Rgb(17, 38, 33),
            window_border: Color::Rgb(88, 174, 145),
            window_title: Color::Rgb(210, 246, 218),
            window_shadow: Color::Rgb(4, 12, 10),
            map_window_bg: Color::Rgb(12, 24, 20),
            panel_window_bg: Color::Rgb(15, 34, 30),
            budget_window_bg: Color::Rgb(14, 31, 28),
            inspect_window_bg: Color::Rgb(14, 29, 25),
            popup_bg: Color::Rgb(20, 43, 37),
            popup_border: Color::Rgb(126, 219, 177),
            popup_title: Color::Rgb(182, 255, 201),
            text_primary: Color::Rgb(218, 243, 225),
            text_secondary: Color::Rgb(166, 206, 191),
            text_muted: Color::Rgb(117, 158, 144),
            text_dim: Color::Rgb(79, 115, 104),
            accent: Color::Rgb(112, 224, 163),
            accent_soft: Color::Rgb(94, 204, 198),
            selection_bg: Color::Rgb(68, 151, 113),
            selection_fg: Color::Rgb(239, 249, 242),
            success: Color::Rgb(139, 231, 140),
            danger: Color::Rgb(230, 118, 122),
            warning: Color::Rgb(233, 201, 108),
            info: Color::Rgb(109, 216, 208),
            sector_residential: Color::Rgb(148, 240, 146),
            sector_residential_bg: Color::Rgb(23, 72, 39),
            sector_commercial: Color::Rgb(119, 214, 235),
            sector_commercial_bg: Color::Rgb(18, 58, 83),
            sector_industrial: Color::Rgb(227, 205, 112),
            sector_industrial_bg: Color::Rgb(83, 71, 20),
            menu_bg: Color::Rgb(182, 221, 196),
            menu_fg: Color::Rgb(16, 38, 31),
            menu_focus_bg: Color::Rgb(69, 135, 105),
            menu_focus_fg: Color::Rgb(239, 248, 241),
            menu_hotkey: Color::Rgb(60, 93, 74),
            menu_right: Color::Rgb(63, 91, 76),
            menu_title: Color::Rgb(30, 76, 60),
            toolbar_bg: Color::Rgb(15, 33, 29),
            toolbar_header: Color::Rgb(194, 236, 205),
            toolbar_rule: Color::Rgb(74, 114, 95),
            toolbar_button_bg: Color::Rgb(33, 56, 49),
            toolbar_button_fg: Color::Rgb(221, 241, 225),
            toolbar_active_bg: Color::Rgb(113, 225, 164),
            toolbar_active_fg: Color::Rgb(13, 31, 26),
            toolbar_armed_bg: Color::Rgb(71, 150, 120),
            status_bg: Color::Rgb(17, 34, 29),
            status_sep: Color::Rgb(70, 110, 92),
            status_city: Color::Rgb(160, 243, 185),
            status_population: Color::Rgb(137, 226, 206),
            status_date: Color::Rgb(212, 235, 219),
            status_message: Color::Rgb(233, 202, 110),
            status_button_run_bg: Color::Rgb(74, 148, 95),
            status_button_run_fg: Color::Rgb(239, 248, 241),
            status_button_pause_bg: Color::Rgb(61, 109, 116),
            status_button_pause_fg: Color::Rgb(205, 243, 244),
            news_ticker_bg: Color::Rgb(12, 28, 24),
            news_ticker_label_bg: Color::Rgb(69, 135, 105),
            news_ticker_label_fg: Color::Rgb(239, 248, 241),
            news_ticker_text: Color::Rgb(220, 243, 226),
            news_ticker_alert: Color::Rgb(233, 201, 108),
            input_bg: Color::Rgb(36, 60, 52),
            input_fg: Color::Rgb(229, 244, 232),
            input_focus_bg: Color::Rgb(113, 225, 164),
            input_focus_fg: Color::Rgb(12, 29, 25),
            slider_bg: Color::Rgb(52, 82, 70),
            slider_water_fg: Color::Rgb(102, 205, 228),
            slider_water_focus_bg: Color::Rgb(74, 98, 92),
            slider_trees_fg: Color::Rgb(147, 228, 118),
            slider_trees_focus_bg: Color::Rgb(71, 98, 73),
            button_bg: Color::Rgb(24, 49, 42),
            button_fg: Color::Rgb(221, 241, 225),
            button_focus_bg: Color::Rgb(113, 225, 164),
            button_focus_fg: Color::Rgb(12, 29, 25),
            button_armed_bg: Color::Rgb(68, 146, 116),
            scrollbar_button_fg: Color::Rgb(15, 37, 30),
            scrollbar_button_bg: Color::Rgb(182, 221, 196),
            scrollbar_track_fg: Color::Rgb(66, 95, 81),
            scrollbar_track_bg: Color::Rgb(27, 45, 39),
            scrollbar_thumb_fg: Color::Rgb(235, 246, 238),
            scrollbar_thumb_bg: Color::Rgb(86, 138, 114),
            scrollbar_corner_fg: Color::Rgb(66, 95, 81),
            scrollbar_corner_bg: Color::Rgb(182, 221, 196),
            preview_valid_fg: Color::Rgb(160, 246, 189),
            preview_valid_bg: Color::Rgb(20, 88, 60),
            preview_invalid_fg: Color::Rgb(255, 165, 159),
            preview_invalid_bg: Color::Rgb(93, 31, 36),
            preview_line_fg: Color::Rgb(151, 227, 221),
            preview_line_bg: Color::Rgb(24, 78, 73),
            viewport_outline: Color::Rgb(160, 243, 185),
            disaster_bg: Color::Rgb(49, 24, 25),
            disaster_border: Color::Rgb(200, 98, 94),
            disaster_select_bg: Color::Rgb(84, 41, 42),
        },
        ThemePreset::Candy => {
            let base = palette_for(ThemePreset::Sunset);
            UiPalette {
                desktop_bg: Color::Rgb(33, 20, 47),
                title: Color::Rgb(255, 231, 122),
                subtitle: Color::Rgb(255, 178, 216),
                window_bg: Color::Rgb(55, 29, 74),
                window_border: Color::Rgb(255, 120, 192),
                window_title: Color::Rgb(255, 236, 184),
                popup_border: Color::Rgb(255, 154, 120),
                popup_title: Color::Rgb(255, 224, 137),
                accent: Color::Rgb(255, 166, 102),
                accent_soft: Color::Rgb(120, 225, 239),
                selection_bg: Color::Rgb(255, 215, 109),
                selection_fg: Color::Rgb(58, 24, 58),
                info: Color::Rgb(127, 224, 255),
                menu_bg: Color::Rgb(255, 208, 228),
                menu_fg: Color::Rgb(69, 28, 74),
                menu_focus_bg: Color::Rgb(255, 126, 176),
                menu_focus_fg: Color::Rgb(255, 245, 248),
                menu_hotkey: Color::Rgb(155, 68, 120),
                menu_right: Color::Rgb(134, 74, 110),
                menu_title: Color::Rgb(120, 44, 102),
                toolbar_bg: Color::Rgb(49, 26, 67),
                toolbar_button_bg: Color::Rgb(78, 41, 101),
                toolbar_active_bg: Color::Rgb(255, 214, 109),
                toolbar_active_fg: Color::Rgb(65, 28, 69),
                toolbar_armed_bg: Color::Rgb(255, 123, 173),
                status_city: Color::Rgb(255, 223, 122),
                status_population: Color::Rgb(132, 230, 255),
                status_button_pause_bg: Color::Rgb(104, 78, 155),
                status_button_pause_fg: Color::Rgb(238, 228, 250),
                input_bg: Color::Rgb(87, 46, 109),
                input_focus_bg: Color::Rgb(255, 214, 109),
                input_focus_fg: Color::Rgb(64, 28, 69),
                button_bg: Color::Rgb(69, 35, 89),
                button_focus_bg: Color::Rgb(255, 214, 109),
                button_focus_fg: Color::Rgb(64, 28, 69),
                button_armed_bg: Color::Rgb(255, 120, 171),
                scrollbar_thumb_bg: Color::Rgb(190, 101, 162),
                viewport_outline: Color::Rgb(255, 223, 122),
                disaster_bg: Color::Rgb(76, 24, 45),
                disaster_border: Color::Rgb(255, 111, 140),
                disaster_select_bg: Color::Rgb(128, 42, 68),
                ..base
            }
        }
        ThemePreset::Metro => {
            let base = palette_for(ThemePreset::Emerald);
            UiPalette {
                desktop_bg: Color::Rgb(16, 28, 32),
                title: Color::Rgb(255, 207, 97),
                subtitle: Color::Rgb(134, 235, 209),
                window_bg: Color::Rgb(24, 40, 44),
                window_border: Color::Rgb(108, 200, 184),
                window_title: Color::Rgb(229, 245, 222),
                popup_border: Color::Rgb(255, 158, 88),
                popup_title: Color::Rgb(255, 217, 135),
                accent: Color::Rgb(115, 224, 198),
                accent_soft: Color::Rgb(255, 168, 98),
                selection_bg: Color::Rgb(255, 206, 110),
                selection_fg: Color::Rgb(30, 44, 47),
                warning: Color::Rgb(240, 197, 103),
                menu_bg: Color::Rgb(198, 226, 209),
                menu_fg: Color::Rgb(21, 38, 40),
                menu_focus_bg: Color::Rgb(94, 179, 162),
                menu_focus_fg: Color::Rgb(238, 248, 243),
                menu_hotkey: Color::Rgb(61, 102, 93),
                menu_right: Color::Rgb(77, 92, 83),
                menu_title: Color::Rgb(39, 79, 71),
                toolbar_active_bg: Color::Rgb(255, 206, 110),
                toolbar_active_fg: Color::Rgb(29, 42, 45),
                toolbar_armed_bg: Color::Rgb(255, 160, 93),
                status_city: Color::Rgb(255, 211, 113),
                status_message: Color::Rgb(255, 187, 102),
                input_focus_bg: Color::Rgb(255, 206, 110),
                input_focus_fg: Color::Rgb(29, 42, 45),
                button_focus_bg: Color::Rgb(255, 206, 110),
                button_focus_fg: Color::Rgb(29, 42, 45),
                button_armed_bg: Color::Rgb(255, 157, 89),
                scrollbar_thumb_bg: Color::Rgb(100, 155, 141),
                viewport_outline: Color::Rgb(255, 211, 113),
                disaster_bg: Color::Rgb(55, 28, 25),
                disaster_border: Color::Rgb(224, 109, 91),
                disaster_select_bg: Color::Rgb(99, 45, 39),
                ..base
            }
        }
        ThemePreset::Violet => {
            let base = palette_for(ThemePreset::Harbor);
            UiPalette {
                desktop_bg: Color::Rgb(22, 18, 43),
                title: Color::Rgb(255, 214, 110),
                subtitle: Color::Rgb(170, 195, 255),
                window_bg: Color::Rgb(31, 26, 61),
                window_border: Color::Rgb(149, 125, 238),
                window_title: Color::Rgb(230, 223, 255),
                popup_bg: Color::Rgb(36, 30, 68),
                popup_border: Color::Rgb(255, 140, 109),
                popup_title: Color::Rgb(255, 218, 133),
                accent: Color::Rgb(133, 227, 245),
                accent_soft: Color::Rgb(185, 121, 239),
                selection_bg: Color::Rgb(255, 209, 110),
                selection_fg: Color::Rgb(37, 28, 66),
                menu_bg: Color::Rgb(214, 205, 244),
                menu_fg: Color::Rgb(31, 24, 58),
                menu_focus_bg: Color::Rgb(130, 115, 219),
                menu_focus_fg: Color::Rgb(244, 241, 255),
                menu_hotkey: Color::Rgb(86, 69, 160),
                menu_right: Color::Rgb(81, 72, 121),
                menu_title: Color::Rgb(64, 54, 125),
                toolbar_bg: Color::Rgb(27, 23, 53),
                toolbar_button_bg: Color::Rgb(46, 40, 83),
                toolbar_active_bg: Color::Rgb(255, 209, 110),
                toolbar_active_fg: Color::Rgb(37, 28, 66),
                toolbar_armed_bg: Color::Rgb(184, 117, 236),
                status_city: Color::Rgb(255, 216, 117),
                status_population: Color::Rgb(154, 221, 255),
                status_button_pause_bg: Color::Rgb(103, 84, 178),
                status_button_pause_fg: Color::Rgb(240, 235, 255),
                input_bg: Color::Rgb(57, 49, 100),
                input_focus_bg: Color::Rgb(255, 209, 110),
                input_focus_fg: Color::Rgb(37, 28, 66),
                button_bg: Color::Rgb(39, 33, 73),
                button_focus_bg: Color::Rgb(255, 209, 110),
                button_focus_fg: Color::Rgb(37, 28, 66),
                button_armed_bg: Color::Rgb(181, 114, 232),
                scrollbar_thumb_bg: Color::Rgb(116, 104, 180),
                viewport_outline: Color::Rgb(255, 216, 117),
                disaster_bg: Color::Rgb(59, 24, 36),
                disaster_border: Color::Rgb(227, 104, 122),
                disaster_select_bg: Color::Rgb(103, 44, 64),
                ..base
            }
        }
    };
    vividize_palette(theme, palette)
}

fn vividize_palette(theme: ThemePreset, ui: UiPalette) -> UiPalette {
    match theme {
        ThemePreset::Copper => ui,
        ThemePreset::Harbor => apply_vivid_palette(
            ui,
            (7, 24, 56),
            (34, 170, 255),
            (214, 245, 255),
            (0, 224, 255),
            (0, 125, 255),
            (255, 188, 60),
            (56, 230, 160),
            (255, 95, 95),
            (94, 214, 255),
        ),
        ThemePreset::Sunset => apply_vivid_palette(
            ui,
            (62, 14, 30),
            (255, 116, 70),
            (255, 232, 188),
            (255, 170, 0),
            (255, 64, 128),
            (255, 214, 76),
            (128, 232, 90),
            (255, 90, 90),
            (255, 132, 64),
        ),
        ThemePreset::Emerald => apply_vivid_palette(
            ui,
            (7, 42, 22),
            (38, 208, 112),
            (224, 248, 226),
            (74, 255, 150),
            (0, 188, 170),
            (230, 212, 58),
            (104, 244, 82),
            (255, 104, 104),
            (56, 220, 198),
        ),
        ThemePreset::Candy => apply_vivid_palette(
            ui,
            (50, 8, 58),
            (255, 64, 190),
            (255, 230, 246),
            (255, 116, 40),
            (76, 230, 255),
            (255, 220, 66),
            (104, 255, 134),
            (255, 84, 140),
            (112, 226, 255),
        ),
        ThemePreset::Metro => apply_vivid_palette(
            ui,
            (12, 48, 54),
            (0, 208, 188),
            (224, 246, 240),
            (255, 150, 48),
            (0, 214, 255),
            (255, 214, 82),
            (118, 236, 92),
            (255, 98, 78),
            (86, 226, 255),
        ),
        ThemePreset::Violet => apply_vivid_palette(
            ui,
            (30, 16, 72),
            (120, 94, 255),
            (234, 228, 255),
            (86, 210, 255),
            (234, 76, 255),
            (255, 210, 82),
            (120, 238, 110),
            (255, 92, 132),
            (124, 204, 255),
        ),
    }
}

fn apply_vivid_palette(
    mut ui: UiPalette,
    bg: (u8, u8, u8),
    border: (u8, u8, u8),
    text: (u8, u8, u8),
    accent: (u8, u8, u8),
    accent_soft: (u8, u8, u8),
    selection: (u8, u8, u8),
    success: (u8, u8, u8),
    danger: (u8, u8, u8),
    info: (u8, u8, u8),
) -> UiPalette {
    ui.desktop_bg = scale_tuple(bg, 0.48);
    ui.title = rgb(selection);
    ui.subtitle = rgb(accent_soft);
    ui.window_bg = rgb(bg);
    ui.window_shadow = scale_tuple(bg, 0.24);
    ui.map_window_bg = scale_tuple(bg, 0.62);
    ui.panel_window_bg = scale_tuple(bg, 0.86);
    ui.budget_window_bg = scale_tuple(bg, 0.82);
    ui.inspect_window_bg = scale_tuple(bg, 0.78);
    ui.popup_bg = scale_tuple(bg, 0.92);
    ui.window_border = rgb(border);
    ui.popup_border = rgb(border);
    ui.window_title = rgb(text);
    ui.popup_title = rgb(selection);
    ui.text_primary = rgb(text);
    ui.text_secondary = scale_tuple(text, 0.82);
    ui.text_muted = scale_tuple(text, 0.64);
    ui.text_dim = scale_tuple(text, 0.48);
    ui.accent = rgb(accent);
    ui.accent_soft = rgb(accent_soft);
    ui.selection_bg = rgb(selection);
    ui.selection_fg = scale_tuple(bg, 0.35);
    ui.success = rgb(success);
    ui.danger = rgb(danger);
    ui.warning = rgb(selection);
    ui.info = rgb(info);
    ui.sector_residential = rgb(success);
    ui.sector_residential_bg = scale_tuple(success, 0.32);
    ui.sector_commercial = rgb(info);
    ui.sector_commercial_bg = scale_tuple(info, 0.3);
    ui.sector_industrial = rgb(selection);
    ui.sector_industrial_bg = scale_tuple(selection, 0.28);
    ui.menu_bg = scale_tuple(bg, 0.84);
    ui.menu_fg = rgb(text);
    ui.menu_focus_bg = rgb(accent);
    ui.menu_focus_fg = scale_tuple(bg, 0.35);
    ui.menu_hotkey = rgb(accent_soft);
    ui.menu_right = scale_tuple(text, 0.78);
    ui.menu_title = rgb(selection);
    ui.toolbar_bg = scale_tuple(bg, 0.78);
    ui.toolbar_header = rgb(text);
    ui.toolbar_rule = scale_tuple(border, 0.64);
    ui.toolbar_button_bg = scale_tuple(bg, 0.68);
    ui.toolbar_button_fg = rgb(text);
    ui.toolbar_active_bg = rgb(selection);
    ui.toolbar_active_fg = scale_tuple(bg, 0.35);
    ui.toolbar_armed_bg = rgb(accent_soft);
    ui.status_bg = scale_tuple(bg, 0.8);
    ui.status_sep = scale_tuple(border, 0.58);
    ui.status_city = rgb(selection);
    ui.status_population = rgb(accent_soft);
    ui.status_date = rgb(text);
    ui.status_message = rgb(accent);
    ui.status_button_run_bg = rgb(success);
    ui.status_button_run_fg = scale_tuple(bg, 0.25);
    ui.status_button_pause_bg = rgb(accent_soft);
    ui.status_button_pause_fg = scale_tuple(bg, 0.25);
    ui.news_ticker_bg = scale_tuple(bg, 0.7);
    ui.news_ticker_label_bg = rgb(accent);
    ui.news_ticker_label_fg = scale_tuple(bg, 0.22);
    ui.news_ticker_text = rgb(text);
    ui.news_ticker_alert = rgb(selection);
    ui.input_bg = scale_tuple(bg, 0.72);
    ui.input_fg = rgb(text);
    ui.input_focus_bg = rgb(selection);
    ui.input_focus_fg = scale_tuple(bg, 0.35);
    ui.slider_bg = scale_tuple(bg, 0.9);
    ui.slider_water_fg = rgb(info);
    ui.slider_water_focus_bg = scale_tuple(border, 0.7);
    ui.slider_trees_fg = rgb(success);
    ui.slider_trees_focus_bg = scale_tuple(success, 0.45);
    ui.button_bg = scale_tuple(bg, 0.66);
    ui.button_fg = rgb(text);
    ui.button_focus_bg = rgb(selection);
    ui.button_focus_fg = scale_tuple(bg, 0.35);
    ui.button_armed_bg = rgb(accent_soft);
    ui.scrollbar_button_fg = scale_tuple(bg, 0.35);
    ui.scrollbar_button_bg = rgb(border);
    ui.scrollbar_track_fg = scale_tuple(border, 0.56);
    ui.scrollbar_track_bg = scale_tuple(bg, 0.62);
    ui.scrollbar_thumb_fg = rgb(text);
    ui.scrollbar_thumb_bg = rgb(accent_soft);
    ui.scrollbar_corner_fg = scale_tuple(border, 0.56);
    ui.scrollbar_corner_bg = rgb(border);
    ui.preview_valid_fg = rgb(success);
    ui.preview_valid_bg = scale_tuple(success, 0.36);
    ui.preview_invalid_fg = rgb(danger);
    ui.preview_invalid_bg = scale_tuple(danger, 0.36);
    ui.preview_line_fg = rgb(info);
    ui.preview_line_bg = scale_tuple(info, 0.34);
    ui.viewport_outline = rgb(selection);
    ui.disaster_bg = scale_tuple(danger, 0.34);
    ui.disaster_border = rgb(danger);
    ui.disaster_select_bg = scale_tuple(danger, 0.48);
    ui
}

fn rgb(color: (u8, u8, u8)) -> Color {
    Color::Rgb(color.0, color.1, color.2)
}

fn scale_tuple(color: (u8, u8, u8), factor: f32) -> Color {
    Color::Rgb(
        ((color.0 as f32) * factor).round() as u8,
        ((color.1 as f32) * factor).round() as u8,
        ((color.2 as f32) * factor).round() as u8,
    )
}

fn copper_palette() -> UiPalette {
    UiPalette {
        desktop_bg: Color::Rgb(24, 18, 14),
        title: Color::Rgb(242, 181, 76),
        subtitle: Color::Rgb(157, 187, 170),
        window_bg: Color::Rgb(33, 26, 21),
        window_border: Color::Rgb(150, 122, 83),
        window_title: Color::Rgb(238, 216, 172),
        window_shadow: Color::Rgb(12, 9, 7),
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
        news_ticker_bg: Color::Rgb(27, 21, 18),
        news_ticker_label_bg: Color::Rgb(111, 70, 28),
        news_ticker_label_fg: Color::Rgb(249, 238, 214),
        news_ticker_text: Color::Rgb(236, 223, 196),
        news_ticker_alert: Color::Rgb(243, 208, 117),
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
#[allow(dead_code)]
pub fn lerp_color(val: u8, low: (u8, u8, u8), high: (u8, u8, u8)) -> Color {
    let f = val as f32 / 255.0;
    let r = (low.0 as f32 + (high.0 as f32 - low.0 as f32) * f) as u8;
    let g = (low.1 as f32 + (high.1 as f32 - low.1 as f32) * f) as u8;
    let b = (low.2 as f32 + (high.2 as f32 - low.2 as f32) * f) as u8;
    Color::Rgb(r, g, b)
}

/// Interpolates through three RGB stops: low (0) → mid (128) → high (255).
pub fn lerp_color3(val: u8, low: (u8, u8, u8), mid: (u8, u8, u8), high: (u8, u8, u8)) -> Color {
    if val < 128 {
        let f = val as f32 / 127.0;
        let r = (low.0 as f32 + (mid.0 as f32 - low.0 as f32) * f) as u8;
        let g = (low.1 as f32 + (mid.1 as f32 - low.1 as f32) * f) as u8;
        let b = (low.2 as f32 + (mid.2 as f32 - low.2 as f32) * f) as u8;
        Color::Rgb(r, g, b)
    } else {
        let f = (val - 128) as f32 / 127.0;
        let r = (mid.0 as f32 + (high.0 as f32 - mid.0 as f32) * f) as u8;
        let g = (mid.1 as f32 + (high.1 as f32 - mid.1 as f32) * f) as u8;
        let b = (mid.2 as f32 + (high.2 as f32 - mid.2 as f32) * f) as u8;
        Color::Rgb(r, g, b)
    }
}

/// Legend metadata for an overlay: gradient stops and edge labels.
pub struct OverlayLegendInfo {
    pub title: &'static str,
    pub low: (u8, u8, u8),
    pub mid: (u8, u8, u8),
    pub high: (u8, u8, u8),
    pub low_label: &'static str,
    pub high_label: &'static str,
}

/// Returns the legend info for a given overlay mode, or `None` for `OverlayMode::None`.
pub fn overlay_legend_info(mode: OverlayMode) -> Option<OverlayLegendInfo> {
    match mode {
        OverlayMode::None => None,
        OverlayMode::Power => Some(OverlayLegendInfo {
            title: "Power Grid",
            low: (90, 20, 10),
            mid: (160, 130, 20),
            high: (20, 160, 40),
            low_label: "0%",
            high_label: "100%",
        }),
        OverlayMode::Water => Some(OverlayLegendInfo {
            title: "Water Service",
            low: (80, 40, 10),
            mid: (20, 110, 130),
            high: (20, 100, 210),
            low_label: "Dry",
            high_label: "Full",
        }),
        OverlayMode::Traffic => Some(OverlayLegendInfo {
            title: "Traffic",
            low: (20, 120, 20),
            mid: (170, 140, 10),
            high: (180, 20, 10),
            low_label: "Clear",
            high_label: "Gridlock",
        }),
        OverlayMode::Pollution => Some(OverlayLegendInfo {
            title: "Pollution",
            low: (20, 120, 20),
            mid: (170, 140, 10),
            high: (160, 30, 10),
            low_label: "Clean",
            high_label: "Severe",
        }),
        OverlayMode::LandValue => Some(OverlayLegendInfo {
            title: "Land Value",
            low: (60, 30, 20),
            mid: (130, 110, 20),
            high: (20, 150, 50),
            low_label: "Low",
            high_label: "High",
        }),
        OverlayMode::Crime => Some(OverlayLegendInfo {
            title: "Crime Rate",
            low: (20, 120, 20),
            mid: (160, 110, 10),
            high: (160, 10, 10),
            low_label: "Safe",
            high_label: "High",
        }),
        OverlayMode::FireRisk => Some(OverlayLegendInfo {
            title: "Fire Risk",
            low: (20, 120, 40),
            mid: (170, 110, 10),
            high: (190, 30, 0),
            low_label: "Safe",
            high_label: "High",
        }),
    }
}

/// Rescale a raw value from `[data_min, data_max]` to the full `[0, 255]` gradient
/// range so that heatmap colors span visibly even when sim values cluster in a
/// narrow band.  Values outside the range are clamped.
fn rescale(raw: u8, data_min: u8, data_max: u8) -> u8 {
    if data_max <= data_min {
        return 0;
    }
    let clamped = raw.clamp(data_min, data_max);
    let ratio = (clamped - data_min) as f32 / (data_max - data_min) as f32;
    (ratio * 255.0).round() as u8
}

/// Returns a background color tint for the given overlay mode and tile overlay.
/// Returns `None` when overlay is `None` (no tinting).
///
/// Each overlay rescales its raw simulation value to the full gradient range so
/// that colour differences are visible even when sim values cluster narrowly.
pub fn overlay_tint(mode: OverlayMode, o: TileOverlay) -> Option<Color> {
    match mode {
        OverlayMode::None => None,
        OverlayMode::Power => Some(lerp_color3(
            o.power_level,
            (90, 20, 10),
            (160, 130, 20),
            (20, 160, 40),
        )),
        OverlayMode::Water => Some(lerp_color3(
            o.water_service,
            (80, 40, 10),
            (20, 110, 130),
            (20, 100, 210),
        )),
        OverlayMode::Traffic => Some(lerp_color3(
            rescale(o.traffic, 0, 120),
            (20, 120, 20),
            (170, 140, 10),
            (180, 20, 10),
        )),
        OverlayMode::Pollution => Some(lerp_color3(
            rescale(o.pollution, 0, 180),
            (20, 120, 20),
            (170, 140, 10),
            (160, 30, 10),
        )),
        OverlayMode::LandValue => Some(lerp_color3(
            rescale(o.land_value, 0, 200),
            (60, 30, 20),
            (130, 110, 20),
            (20, 150, 50),
        )),
        OverlayMode::Crime => Some(lerp_color3(
            rescale(o.crime, 0, 100),
            (20, 120, 20),
            (160, 110, 10),
            (160, 10, 10),
        )),
        OverlayMode::FireRisk => Some(lerp_color3(
            rescale(o.fire_risk, 0, 120),
            (20, 120, 40),
            (170, 110, 10),
            (190, 30, 0),
        )),
    }
}

// ── Feature 3: Tile Assets ────────────────────────────────────────────────────

pub struct TileGlyph {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileSprite {
    pub left: SpriteCell,
    pub right: SpriteCell,
}

impl TileSprite {
    pub fn uniform(ch: char, fg: Color, bg: Color) -> Self {
        Self::pair(ch, ch, fg, bg)
    }

    pub fn pair(left: char, right: char, fg: Color, bg: Color) -> Self {
        Self {
            left: SpriteCell { ch: left, fg, bg },
            right: SpriteCell { ch: right, fg, bg },
        }
    }

    pub fn with_bg(self, bg: Color) -> Self {
        Self {
            left: SpriteCell { bg, ..self.left },
            right: SpriteCell { bg, ..self.right },
        }
    }

    pub fn recolor(self, fg: Color, bg: Color) -> Self {
        Self {
            left: SpriteCell {
                fg,
                bg,
                ..self.left
            },
            right: SpriteCell {
                fg,
                bg,
                ..self.right
            },
        }
    }
}

pub fn tile_glyph(tile: Tile, overlay: TileOverlay) -> TileGlyph {
    let ui = ui_palette();
    if overlay.on_fire {
        return TileGlyph {
            ch: '*',
            fg: Color::Rgb(255, 200, 0),
            bg: Color::Rgb(150, 40, 0),
        };
    }

    match tile {
        Tile::Grass => TileGlyph {
            ch: '.',
            fg: Color::Rgb(40, 100, 40),
            bg: Color::Rgb(25, 60, 25),
        },
        Tile::Water => TileGlyph {
            ch: '~',
            fg: Color::Rgb(100, 150, 255),
            bg: Color::Rgb(20, 40, 100),
        },
        Tile::Trees => TileGlyph {
            ch: '^',
            fg: Color::Rgb(30, 180, 30),
            bg: Color::Rgb(20, 50, 20),
        },
        Tile::Dirt => TileGlyph {
            ch: '░',
            fg: Color::Rgb(100, 80, 60),
            bg: Color::Rgb(60, 45, 35),
        },
        Tile::Road | Tile::RoadPowerLine => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(180, 180, 180),
            bg: Color::Rgb(40, 40, 45),
        },
        Tile::Highway => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(240, 220, 140),
            bg: Color::Rgb(55, 50, 30),
        },
        Tile::Onramp => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(220, 210, 130),
            bg: Color::Rgb(50, 45, 30),
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
        Tile::WaterPipe => {
            // Grey when dry (no water service), blue when connected.
            if overlay.water_service > 0 {
                TileGlyph {
                    ch: ' ',
                    fg: Color::Rgb(120, 210, 255),
                    bg: Color::Rgb(18, 42, 60),
                }
            } else {
                TileGlyph {
                    ch: ' ',
                    fg: Color::Rgb(130, 130, 130),
                    bg: Color::Rgb(35, 35, 40),
                }
            }
        }
        Tile::SubwayTunnel => TileGlyph {
            ch: ' ',
            fg: Color::Rgb(220, 220, 220),
            bg: Color::Rgb(28, 28, 28),
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
            ch: 'r',
            fg: ui.sector_residential,
            bg: ui.sector_residential_bg,
        },
        Tile::ResMed => TileGlyph {
            ch: 'R',
            fg: ui.sector_residential,
            bg: ui.sector_residential_bg,
        },
        Tile::ResHigh => TileGlyph {
            ch: '#',
            fg: Color::Rgb(208, 245, 202),
            bg: ui.sector_residential_bg,
        },
        Tile::CommLow => TileGlyph {
            ch: 'c',
            fg: ui.sector_commercial,
            bg: ui.sector_commercial_bg,
        },
        Tile::CommHigh => TileGlyph {
            ch: 'C',
            fg: Color::Rgb(204, 232, 250),
            bg: ui.sector_commercial_bg,
        },
        Tile::IndLight => TileGlyph {
            ch: 'i',
            fg: ui.sector_industrial,
            bg: ui.sector_industrial_bg,
        },
        Tile::IndHeavy => TileGlyph {
            ch: 'I',
            fg: Color::Rgb(245, 225, 172),
            bg: ui.sector_industrial_bg,
        },
        Tile::PowerPlantCoal | Tile::PowerPlantGas => TileGlyph {
            ch: '@',
            fg: Color::Rgb(255, 255, 100),
            bg: Color::Rgb(60, 60, 30),
        },
        Tile::Park => TileGlyph {
            ch: '"',
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
        Tile::BusDepot => TileGlyph {
            ch: 'B',
            fg: Color::Rgb(250, 240, 170),
            bg: Color::Rgb(90, 65, 15),
        },
        Tile::RailDepot => TileGlyph {
            ch: 'R',
            fg: Color::Rgb(230, 210, 185),
            bg: Color::Rgb(75, 50, 35),
        },
        Tile::SubwayStation => TileGlyph {
            ch: 'U',
            fg: Color::Rgb(215, 230, 250),
            bg: Color::Rgb(25, 55, 95),
        },
        Tile::WaterPump => TileGlyph {
            ch: 'W',
            fg: Color::Rgb(200, 250, 255),
            bg: Color::Rgb(20, 70, 90),
        },
        Tile::WaterTower => TileGlyph {
            ch: 'T',
            fg: Color::Rgb(210, 250, 255),
            bg: Color::Rgb(30, 65, 78),
        },
        Tile::WaterTreatment => TileGlyph {
            ch: 'C',
            fg: Color::Rgb(220, 255, 240),
            bg: Color::Rgb(25, 85, 60),
        },
        Tile::Desalination => TileGlyph {
            ch: 'D',
            fg: Color::Rgb(220, 250, 255),
            bg: Color::Rgb(25, 70, 110),
        },
        Tile::PowerPlantNuclear => TileGlyph {
            ch: 'N',
            fg: Color::Rgb(180, 255, 180),
            bg: Color::Rgb(20, 50, 20),
        },
        Tile::PowerPlantWind => TileGlyph {
            ch: 'W',
            fg: Color::Rgb(200, 240, 255),
            bg: Color::Rgb(30, 50, 80),
        },
        Tile::PowerPlantSolar => TileGlyph {
            ch: 'S',
            fg: Color::Rgb(255, 240, 100),
            bg: Color::Rgb(50, 40, 10),
        },
        Tile::School => TileGlyph {
            ch: 'S',
            fg: Color::Rgb(255, 255, 180),
            bg: Color::Rgb(90, 70, 10),
        },
        Tile::Stadium => TileGlyph {
            ch: 'O',
            fg: Color::Rgb(200, 255, 200),
            bg: Color::Rgb(10, 60, 10),
        },
        Tile::Library => TileGlyph {
            ch: 'L',
            fg: Color::Rgb(255, 200, 150),
            bg: Color::Rgb(80, 45, 15),
        },
        Tile::Rubble => TileGlyph {
            ch: '░',
            fg: Color::Rgb(80, 80, 80),
            bg: Color::Rgb(40, 40, 40),
        },
    }
}

pub fn tile_sprite(tile: Tile, overlay: TileOverlay) -> TileSprite {
    let glyph = tile_glyph(tile, overlay);

    if overlay.on_fire {
        return TileSprite::uniform(glyph.ch, glyph.fg, glyph.bg);
    }

    let (left, right) = match tile {
        Tile::ResLow => ('/', '\\'),
        Tile::ResMed => ('[', ']'),
        Tile::ResHigh => ('|', '|'),
        Tile::CommLow => ('(', ')'),
        Tile::CommHigh => ('{', '}'),
        Tile::IndLight => ('/', '|'),
        Tile::IndHeavy => ('|', 'T'),
        Tile::Police => ('P', ']'),
        Tile::Fire => ('F', ']'),
        Tile::Hospital => ('[', 'H'),
        Tile::BusDepot => ('[', 'B'),
        Tile::RailDepot => ('[', 'R'),
        Tile::WaterPump => ('[', 'W'),
        Tile::WaterTower => ('[', 'T'),
        Tile::WaterTreatment => ('[', 'C'),
        Tile::Desalination => ('[', 'D'),
        Tile::PowerPlantCoal | Tile::PowerPlantGas => ('@', '@'),
        Tile::PowerPlantNuclear => ('[', 'N'),
        Tile::PowerPlantWind => ('(', ')'),
        Tile::PowerPlantSolar => ('[', 'S'),
        Tile::School => ('[', 'S'),
        Tile::Stadium => ('[', 'O'),
        Tile::Library => ('[', 'L'),
        _ => (glyph.ch, glyph.ch),
    };

    TileSprite::pair(left, right, glyph.fg, glyph.bg)
}

// ── Feature 3b: Multi-tile building art ──────────────────────────────────────

/// Footprint size (width, height) for multi-tile buildings.
/// Mirrors `Tool::footprint()` but keyed on tile type for the renderer.
pub fn tile_footprint_size(tile: Tile) -> (usize, usize) {
    match tile {
        Tile::PowerPlantCoal | Tile::PowerPlantGas | Tile::PowerPlantNuclear | Tile::Stadium => {
            (4, 4)
        }
        Tile::Police | Tile::Fire => (3, 3),
        Tile::WaterTreatment | Tile::Desalination => (3, 3),
        Tile::WaterTower
        | Tile::Park
        | Tile::BusDepot
        | Tile::RailDepot
        | Tile::PowerPlantSolar => (2, 2),
        _ => (1, 1),
    }
}

// Per-position (left_char, right_char) for each tile in a building, row-major.
// Index = dy * width + dx.

const POLICE_ART: [(char, char); 9] = [
    ('┌', '─'),
    ('P', 'D'),
    ('─', '┐'),
    ('│', ' '),
    ('*', '*'),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const FIRE_ART: [(char, char); 9] = [
    ('┌', '─'),
    ('F', 'D'),
    ('─', '┐'),
    ('│', ' '),
    ('*', '*'),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const WATER_TREATMENT_ART: [(char, char); 9] = [
    ('┌', '─'),
    ('W', 'T'),
    ('─', '┐'),
    ('│', ' '),
    ('~', '~'),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const DESALINATION_ART: [(char, char); 9] = [
    ('┌', '─'),
    ('D', 'S'),
    ('─', '┐'),
    ('│', ' '),
    ('~', '~'),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const COAL_PLANT_ART: [(char, char); 16] = [
    ('┌', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┐'),
    ('│', ' '),
    ('C', 'O'),
    ('A', 'L'),
    (' ', '│'),
    ('│', ' '),
    (' ', '@'),
    ('@', ' '),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const GAS_PLANT_ART: [(char, char); 16] = [
    ('┌', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┐'),
    ('│', ' '),
    ('G', 'A'),
    ('S', ' '),
    (' ', '│'),
    ('│', ' '),
    (' ', '@'),
    ('@', ' '),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const NUCLEAR_PLANT_ART: [(char, char); 16] = [
    ('┌', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┐'),
    ('│', ' '),
    ('N', 'U'),
    ('C', ' '),
    (' ', '│'),
    ('│', ' '),
    (' ', '☢'),
    ('☢', ' '),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const SOLAR_PLANT_ART: [(char, char); 4] = [('┌', 'S'), ('L', '┐'), ('└', '─'), ('─', '┘')];

const STADIUM_ART: [(char, char); 16] = [
    ('┌', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┐'),
    ('│', '('),
    (' ', ' '),
    (' ', ')'),
    (' ', '│'),
    ('│', '('),
    (' ', ' '),
    (' ', ')'),
    (' ', '│'),
    ('└', '─'),
    ('─', '─'),
    ('─', '─'),
    ('─', '┘'),
];

const PARK_ART: [(char, char); 4] = [('"', '^'), ('^', '"'), ('"', '.'), ('.', '"')];

const WATER_TOWER_ART: [(char, char); 4] = [('[', '='), ('=', ']'), ('|', ' '), (' ', '|')];

const BUS_DEPOT_ART: [(char, char); 4] = [('┌', 'B'), ('D', '┐'), ('└', '─'), ('─', '┘')];

const RAIL_DEPOT_ART: [(char, char); 4] = [('┌', 'R'), ('D', '┐'), ('└', '─'), ('─', '┘')];

/// Returns per-position character art for multi-tile buildings, or `None` for 1×1 tiles.
pub fn building_art(tile: Tile) -> Option<&'static [(char, char)]> {
    match tile {
        Tile::Police => Some(&POLICE_ART),
        Tile::Fire => Some(&FIRE_ART),
        Tile::WaterTreatment => Some(&WATER_TREATMENT_ART),
        Tile::Desalination => Some(&DESALINATION_ART),
        Tile::PowerPlantCoal => Some(&COAL_PLANT_ART),
        Tile::PowerPlantGas => Some(&GAS_PLANT_ART),
        Tile::PowerPlantNuclear => Some(&NUCLEAR_PLANT_ART),
        Tile::PowerPlantSolar => Some(&SOLAR_PLANT_ART),
        Tile::Stadium => Some(&STADIUM_ART),
        Tile::Park => Some(&PARK_ART),
        Tile::WaterTower => Some(&WATER_TOWER_ART),
        Tile::BusDepot => Some(&BUS_DEPOT_ART),
        Tile::RailDepot => Some(&RAIL_DEPOT_ART),
        _ => None,
    }
}

// ── Feature 4: Network characters (Borders) ──────────────────────────────────

fn network_sprite_chars(tile: Tile, n: bool, e: bool, s: bool, w: bool) -> (char, char) {
    let idx = (if n { 1 } else { 0 })
        | (if e { 2 } else { 0 })
        | (if s { 4 } else { 0 })
        | (if w { 8 } else { 0 });

    match tile {
        Tile::Road | Tile::RoadPowerLine | Tile::Onramp => [
            (' ', ' '),
            (' ', '║'),
            ('═', '═'),
            (' ', '╚'),
            (' ', '║'),
            (' ', '║'),
            (' ', '╔'),
            (' ', '╠'),
            ('═', '═'),
            ('═', '╝'),
            ('═', '═'),
            ('═', '╩'),
            ('═', '╗'),
            ('═', '╣'),
            ('═', '╦'),
            ('═', '╬'),
        ][idx],
        Tile::Highway => [
            (' ', ' '),
            (' ', '║'),
            ('█', '█'),
            (' ', '╚'),
            (' ', '║'),
            (' ', '║'),
            (' ', '╔'),
            (' ', '╠'),
            ('█', '█'),
            ('█', '╝'),
            ('█', '█'),
            ('█', '╩'),
            ('█', '╗'),
            ('█', '╣'),
            ('█', '╦'),
            ('█', '╬'),
        ][idx],
        Tile::Rail => [
            (' ', ' '),
            (' ', '╽'),
            ('━', '━'),
            (' ', '┗'),
            (' ', '╿'),
            (' ', '┃'),
            (' ', '┏'),
            (' ', '┣'),
            ('━', '━'),
            ('━', '┛'),
            ('━', '━'),
            ('━', '┻'),
            ('━', '┓'),
            ('━', '┫'),
            ('━', '┳'),
            ('━', '╋'),
        ][idx],
        Tile::PowerLine => [
            (' ', ' '),
            (' ', '╏'),
            ('╌', '╌'),
            (' ', '┗'),
            (' ', '╏'),
            (' ', '┃'),
            (' ', '┏'),
            (' ', '┣'),
            ('╌', '╌'),
            ('╌', '┛'),
            ('╌', '╌'),
            ('╌', '┻'),
            ('╌', '┓'),
            ('╌', '┫'),
            ('╌', '┳'),
            ('╌', '╋'),
        ][idx],
        Tile::WaterPipe => [
            (' ', ' '),
            (' ', '│'),
            ('─', '─'),
            (' ', '└'),
            (' ', '│'),
            (' ', '│'),
            (' ', '┌'),
            (' ', '├'),
            ('─', '─'),
            ('─', '┘'),
            ('─', '─'),
            ('─', '┴'),
            ('─', '┐'),
            ('─', '┤'),
            ('─', '┬'),
            ('─', '┼'),
        ][idx],
        Tile::SubwayTunnel => [
            (' ', ' '),
            (' ', '║'),
            ('═', '═'),
            (' ', '╚'),
            (' ', '║'),
            (' ', '║'),
            (' ', '╔'),
            (' ', '╠'),
            ('═', '═'),
            ('═', '╝'),
            ('═', '═'),
            ('═', '╩'),
            ('═', '╗'),
            ('═', '╣'),
            ('═', '╦'),
            ('═', '╬'),
        ][idx],
        _ => (' ', ' '),
    }
}

pub fn network_sprite(
    tile: Tile,
    n: bool,
    e: bool,
    s: bool,
    w: bool,
    fg: Color,
    bg: Color,
) -> TileSprite {
    let (left, right) = network_sprite_chars(tile, n, e, s, w);
    TileSprite::pair(left, right, fg, bg)
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
        for _ in 0..8 {
            m = m.next();
        }
        assert_eq!(m, OverlayMode::None);
    }

    #[test]
    fn overlay_tint_none_returns_none() {
        assert!(overlay_tint(OverlayMode::None, TileOverlay::default()).is_none());
    }

    #[test]
    fn overlay_tint_power_unpowered_is_red() {
        let overlay = TileOverlay {
            power_level: 0,
            ..TileOverlay::default()
        };
        assert_eq!(
            overlay_tint(OverlayMode::Power, overlay),
            Some(Color::Rgb(90, 20, 10))
        );
    }

    #[test]
    fn overlay_tint_power_powered_is_greenish() {
        let overlay = TileOverlay {
            power_level: 255,
            ..TileOverlay::default()
        };
        assert_eq!(
            overlay_tint(OverlayMode::Power, overlay),
            Some(Color::Rgb(20, 160, 40))
        );
    }

    #[test]
    fn ui_palette_exposes_expected_core_colors() {
        let palette = palette_for(ThemePreset::Copper);
        assert_eq!(palette.menu_fg, Color::Rgb(31, 19, 10));
        assert_eq!(palette.toolbar_active_bg, Color::Rgb(233, 176, 80));
        assert_eq!(palette.scrollbar_thumb_bg, Color::Rgb(128, 96, 55));
        assert_eq!(palette.sector_residential, Color::Rgb(130, 218, 122));
        assert_eq!(palette.sector_commercial, Color::Rgb(110, 196, 232));
        assert_eq!(palette.sector_industrial, Color::Rgb(227, 194, 96));
    }

    #[test]
    fn network_sprite_keeps_vertical_road_single_stem() {
        let sprite = network_sprite(
            Tile::Road,
            true,
            false,
            true,
            false,
            Color::White,
            Color::Black,
        );
        assert_eq!(sprite.left.ch, ' ');
        assert_eq!(sprite.right.ch, '║');
    }

    #[test]
    fn network_sprite_uses_single_stem_crossing() {
        let sprite = network_sprite(
            Tile::PowerLine,
            true,
            true,
            true,
            true,
            Color::White,
            Color::Black,
        );
        assert_eq!(sprite.left.ch, '╌');
        assert_eq!(sprite.right.ch, '╋');
    }

    #[test]
    fn fire_and_tower_glyphs_use_single_cell_markers() {
        let fire = tile_glyph(
            Tile::Grass,
            TileOverlay {
                on_fire: true,
                ..TileOverlay::default()
            },
        );
        let tower = tile_glyph(Tile::ResHigh, TileOverlay::default());

        assert_eq!(fire.ch, '*');
        assert_eq!(tower.ch, '#');
    }

    #[test]
    fn developed_zone_glyphs_are_distinct_from_empty_zones() {
        let empty_res = tile_glyph(Tile::ZoneRes, TileOverlay::default());
        let low_res = tile_glyph(Tile::ResLow, TileOverlay::default());
        let low_comm = tile_glyph(Tile::CommLow, TileOverlay::default());
        let light_ind = tile_glyph(Tile::IndLight, TileOverlay::default());

        assert_eq!(empty_res.ch, '▒');
        assert_eq!(low_res.ch, 'r');
        assert_eq!(low_comm.ch, 'c');
        assert_eq!(light_ind.ch, 'i');
    }
}
