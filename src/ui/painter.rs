//! # InGamePainter trait
//!
//! This module defines the rendering contract that every in-game frontend style must implement.
//! The shared orchestrator (`orchestrate_ingame`) drives the painter methods in the
//! correct order, ensuring every supported presentation mode renders the same UI elements.
//!
//! ## Architecture
//!
//! ```text
//!   InGameDesktopView (shared view model)
//!          │
//!          ▼
//!   orchestrate_ingame()  ← shared logic, lives here
//!          │
//!          ├─► paint_map()
//!          ├─► paint_menu_bar()
//!          ├─► paint_status_bar()
//!          ├─► ...
//!          │
//!          ▼
//!   UiAreas populated from returned click areas
//! ```
//!
//! Adding a new UI element:
//! 1. Add a method to `InGamePainter` — compiler error in each painter implementation
//! 2. Add orchestration call in `orchestrate_ingame` — compiles only when both impls exist
//! 3. Wire up any returned click areas to `UiAreas`

use crate::{
    app::{camera::Camera, screens::InGameScreen, ClickArea, MapUiAreas},
    core::{
        map::{Map, ViewLayer},
        sim::SimState,
        tool::Tool,
    },
    ui::{
        runtime::{DesktopLayout, ToolbarHitArea, WindowId},
        theme::OverlayMode,
        view::{
            AdvisorViewModel, BudgetViewModel, ConfirmDialogViewModel, InGameDesktopView,
            NewsTickerViewModel, NewspaperViewModel, StatisticsWindowViewModel,
            TextWindowViewModel, ToolChooserViewModel, ToolbarPaletteViewModel,
        },
    },
};

// ── Click area outputs ──────────────────────────────────────────────────────

/// Click areas produced by `paint_menu_bar`.
#[derive(Default)]
pub struct MenuBarAreas {
    pub menu_bar: ClickArea,
    pub menu_items: [ClickArea; 6],
}

/// Click areas produced by `paint_menu_popup`.
#[derive(Default)]
pub struct MenuPopupAreas {
    pub menu_popup: ClickArea,
    pub menu_popup_items: Vec<ClickArea>,
}

/// Click areas produced by `paint_status_bar`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusBarAreas {
    pub pause_btn: ClickArea,
    pub layer_surface_btn: ClickArea,
    pub layer_underground_btn: ClickArea,
}

/// Click areas produced by `paint_panel_window`.
#[derive(Default)]
pub struct PanelAreas {
    pub toolbar_items: Vec<ToolbarHitArea>,
    pub minimap: ClickArea,
}

/// What kind of preview overlay to render on the map.
pub enum MapPreview<'a> {
    None,
    Line(&'a [(usize, usize)]),
    Rect(&'a [(usize, usize)]),
    /// Footprint tiles + whether all tiles are valid for placement.
    Footprint(&'a [(usize, usize)], bool),
}

// ── Render context ──────────────────────────────────────────────────────────

/// Shared layout info computed once per frame, passed to every painter method.
pub struct FrameLayout {
    pub desktop_layout: DesktopLayout,
    /// Tiles visible horizontally in the map viewport.
    pub view_w: usize,
    /// Tiles visible vertically in the map viewport.
    pub view_h: usize,
    /// Column scale factor in screen cells per map tile.
    pub col_scale: u8,
}

// ── The trait ───────────────────────────────────────────────────────────────

/// Rendering contract for the in-game screen.
///
/// Every in-game frontend style must implement all methods.
/// The shared `orchestrate_ingame()` function calls these methods in order.
///
/// Each method receives only the data it needs (not the entire view model),
/// making dependencies explicit and testable.
pub trait InGamePainter {
    /// Called once at the start of each frame. Set up buffers, clear screen, etc.
    fn begin_frame(&mut self, layout: &FrameLayout);

    /// Render the tile map with overlays, cursor, and animations.
    fn paint_map(
        &mut self,
        map: &Map,
        camera: &Camera,
        overlay_mode: OverlayMode,
        view_layer: ViewLayer,
        current_tool: Tool,
        preview: MapPreview<'_>,
    ) -> MapUiAreas;

    /// Render the top menu bar (TuiCity 2000 | File | Speed | ...).
    fn paint_menu_bar(
        &mut self,
        menu_active: bool,
        menu_selected: usize,
        menu_item_selected: usize,
    ) -> MenuBarAreas;

    /// Render the open menu popup (if menu is active).
    /// `anchor` is the click area of the parent menu item (for positioning).
    fn paint_menu_popup(
        &mut self,
        menu_selected: usize,
        menu_item_selected: usize,
        anchor: ClickArea,
    ) -> MenuPopupAreas;

    /// Render the status bar (city name, treasury, date, pause/layer buttons).
    fn paint_status_bar(
        &mut self,
        sim: &SimState,
        paused: bool,
        view_layer: ViewLayer,
        status_message: Option<&str>,
    ) -> StatusBarAreas;

    /// Render the floating TOOLBOX panel window.
    fn paint_panel_window(
        &mut self,
        toolbar: &ToolbarPaletteViewModel,
        current_tool: Tool,
        sim: &SimState,
        inspect_pos: Option<(usize, usize)>,
        map: &Map,
        ingame: &InGameScreen,
    ) -> PanelAreas;

    /// Render the tool chooser popup (zone/transport/utility selection).
    fn paint_tool_chooser(&mut self, chooser: &ToolChooserViewModel) -> Vec<(ClickArea, Tool)>;

    /// Render a modal confirm dialog (Yes/No/Cancel).
    fn paint_confirm_dialog(&mut self, dialog: &ConfirmDialogViewModel) -> Vec<ClickArea>;

    /// Render the Budget Control Center window.
    fn paint_budget_window(&mut self, budget: &BudgetViewModel, ingame: &InGameScreen);

    /// Render the City Statistics window.
    fn paint_statistics_window(&mut self, stats: &StatisticsWindowViewModel, ingame: &InGameScreen);

    /// Render the Inspect tile-info window.
    fn paint_inspect_window(
        &mut self,
        inspect_pos: Option<(usize, usize)>,
        map: &Map,
        sim: &SimState,
        ingame: &InGameScreen,
    );

    /// Render a scrollable text window (Help, About, or Legend).
    fn paint_text_window(
        &mut self,
        window_id: WindowId,
        view: &TextWindowViewModel,
        ingame: &mut InGameScreen,
    );

    /// Render the City Advisors window.
    fn paint_advisor_window(&mut self, advisor: &AdvisorViewModel, ingame: &InGameScreen);

    /// Render the Newspaper front page. Returns click areas for each section headline.
    fn paint_newspaper_window(
        &mut self,
        newspaper: &NewspaperViewModel,
        ingame: &InGameScreen,
    ) -> Vec<ClickArea>;

    /// Render the news ticker at the bottom of the screen.
    fn paint_news_ticker(&mut self, ticker: &NewsTickerViewModel);

    /// Called once at the end of each frame. Present/flip buffers.
    fn end_frame(&mut self);
}

// ── Shared orchestrator ─────────────────────────────────────────────────────

/// Drive any `InGamePainter` from a view model snapshot.
///
/// This is the single source of truth for "what gets rendered and in what order."
/// All in-game frontends call this instead of implementing their own orchestration.
pub fn orchestrate_ingame(
    painter: &mut impl InGamePainter,
    view: &InGameDesktopView,
    ingame: &mut InGameScreen,
    layout: FrameLayout,
) {
    ingame.ui_areas.desktop = layout.desktop_layout.clone();
    ingame.camera.view_w = layout.view_w;
    ingame.camera.view_h = layout.view_h;
    ingame.camera.col_scale = layout.col_scale;

    painter.begin_frame(&layout);

    // ── Map preview computation ──────────────────────────────────────────
    let (footprint_tiles, footprint_valid) = if Tool::uses_footprint_preview(view.current_tool)
        && view.rect_preview.is_empty()
        && view.line_preview.is_empty()
    {
        let (fw, fh) = view.current_tool.footprint();
        let (cx, cy) = (view.camera.cursor_x, view.camera.cursor_y);
        let ax = cx
            .saturating_sub(fw / 2)
            .min(view.map.width.saturating_sub(fw));
        let ay = cy
            .saturating_sub(fh / 2)
            .min(view.map.height.saturating_sub(fh));
        let tiles: Vec<(usize, usize)> = (0..fh)
            .flat_map(|dy| (0..fw).map(move |dx| (ax + dx, ay + dy)))
            .collect();
        let valid = tiles.iter().all(|&(x, y)| {
            x < view.map.width
                && y < view.map.height
                && view
                    .current_tool
                    .can_place(view.map.view_tile(view.view_layer, x, y))
        });
        (tiles, valid)
    } else {
        (Vec::new(), false)
    };
    let preview = if !view.rect_preview.is_empty() {
        MapPreview::Rect(&view.rect_preview)
    } else if !view.line_preview.is_empty() {
        MapPreview::Line(&view.line_preview)
    } else if !footprint_tiles.is_empty() {
        MapPreview::Footprint(&footprint_tiles, footprint_valid)
    } else {
        MapPreview::None
    };

    // ── Render each element ──────────────────────────────────────────────

    ingame.ui_areas.map = painter.paint_map(
        &view.map,
        &view.camera,
        view.overlay_mode,
        view.view_layer,
        view.current_tool,
        preview,
    );

    let menu_areas = painter.paint_menu_bar(
        view.menu_active,
        view.menu_selected,
        view.menu_item_selected,
    );
    ingame.ui_areas.menu_bar = menu_areas.menu_bar;
    ingame.ui_areas.menu_items = menu_areas.menu_items;

    // Status bar renders before menu popup so the popup draws on top
    let status_areas = painter.paint_status_bar(
        &view.sim,
        view.paused,
        view.view_layer,
        view.status_message.as_deref(),
    );
    ingame.ui_areas.pause_btn = status_areas.pause_btn;
    ingame.ui_areas.layer_surface_btn = status_areas.layer_surface_btn;
    ingame.ui_areas.layer_underground_btn = status_areas.layer_underground_btn;

    // Menu popup (rendered after status bar so it overlays correctly)
    ingame.ui_areas.menu_popup = ClickArea::default();
    ingame.ui_areas.menu_popup_items.clear();
    if view.menu_active {
        let anchor = ingame.ui_areas.menu_items[view.menu_selected];
        let popup_areas =
            painter.paint_menu_popup(view.menu_selected, view.menu_item_selected, anchor);
        ingame.ui_areas.menu_popup = popup_areas.menu_popup;
        ingame.ui_areas.menu_popup_items = popup_areas.menu_popup_items;
    }

    // Panel (TOOLBOX)
    ingame.ui_areas.toolbar_items.clear();
    ingame.ui_areas.minimap = ClickArea::default();
    if ingame.desktop.is_open(WindowId::Panel) {
        let panel_areas = painter.paint_panel_window(
            &view.toolbar,
            view.current_tool,
            &view.sim,
            view.inspect_pos,
            &view.map,
            ingame,
        );
        ingame.ui_areas.toolbar_items = panel_areas.toolbar_items;
        ingame.ui_areas.minimap = panel_areas.minimap;
    }

    // Tool chooser popup
    ingame.ui_areas.tool_chooser_items.clear();
    if let Some(chooser) = &view.tool_chooser {
        ingame.ui_areas.tool_chooser_items = painter.paint_tool_chooser(chooser);
    }

    // Confirm dialog
    ingame.ui_areas.dialog_items.clear();
    if let Some(dialog) = &view.confirm_dialog {
        ingame.ui_areas.dialog_items = painter.paint_confirm_dialog(dialog);
    }

    // Budget window
    if ingame.desktop.is_open(WindowId::Budget) {
        painter.paint_budget_window(&view.budget, ingame);
    }

    // Statistics window
    if let Some(stats) = &view.statistics {
        painter.paint_statistics_window(stats, ingame);
    }

    // Inspect window
    if ingame.desktop.is_open(WindowId::Inspect) {
        painter.paint_inspect_window(view.inspect_pos, &view.map, &view.sim, ingame);
    }

    // Text windows (Help / About / Legend)
    if let Some(help) = &view.help {
        painter.paint_text_window(WindowId::Help, help, ingame);
    }
    if let Some(about) = &view.about {
        painter.paint_text_window(WindowId::About, about, ingame);
    }
    if let Some(legend) = &view.legend {
        painter.paint_text_window(WindowId::Legend, legend, ingame);
    }

    // Advisor window
    if let Some(advisor) = &view.advisor {
        painter.paint_advisor_window(advisor, ingame);
    }

    // Newspaper window
    ingame.ui_areas.newspaper_sections.clear();
    if let Some(newspaper) = &view.newspaper {
        ingame.ui_areas.newspaper_sections = painter.paint_newspaper_window(newspaper, ingame);
    }

    // News ticker
    painter.paint_news_ticker(&view.news_ticker);

    painter.end_frame();
}

// ── Test infrastructure ─────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Records which painter methods were called and in what order.
    /// Use this in tests to verify orchestration logic without any real rendering.
    #[derive(Default)]
    pub struct MockPainter {
        pub calls: Vec<&'static str>,
    }

    impl InGamePainter for MockPainter {
        fn begin_frame(&mut self, _layout: &FrameLayout) {
            self.calls.push("begin_frame");
        }

        fn paint_map(
            &mut self,
            _map: &Map,
            _camera: &Camera,
            _overlay_mode: OverlayMode,
            _view_layer: ViewLayer,
            _current_tool: Tool,
            _preview: MapPreview<'_>,
        ) -> MapUiAreas {
            self.calls.push("paint_map");
            MapUiAreas::default()
        }

        fn paint_menu_bar(
            &mut self,
            _menu_active: bool,
            _menu_selected: usize,
            _menu_item_selected: usize,
        ) -> MenuBarAreas {
            self.calls.push("paint_menu_bar");
            MenuBarAreas::default()
        }

        fn paint_menu_popup(
            &mut self,
            _menu_selected: usize,
            _menu_item_selected: usize,
            _anchor: ClickArea,
        ) -> MenuPopupAreas {
            self.calls.push("paint_menu_popup");
            MenuPopupAreas::default()
        }

        fn paint_status_bar(
            &mut self,
            _sim: &SimState,
            _paused: bool,
            _view_layer: ViewLayer,
            _status_message: Option<&str>,
        ) -> StatusBarAreas {
            self.calls.push("paint_status_bar");
            StatusBarAreas::default()
        }

        fn paint_panel_window(
            &mut self,
            _toolbar: &ToolbarPaletteViewModel,
            _current_tool: Tool,
            _sim: &SimState,
            _inspect_pos: Option<(usize, usize)>,
            _map: &Map,
            _ingame: &InGameScreen,
        ) -> PanelAreas {
            self.calls.push("paint_panel_window");
            PanelAreas::default()
        }

        fn paint_tool_chooser(
            &mut self,
            _chooser: &ToolChooserViewModel,
        ) -> Vec<(ClickArea, Tool)> {
            self.calls.push("paint_tool_chooser");
            Vec::new()
        }

        fn paint_confirm_dialog(&mut self, _dialog: &ConfirmDialogViewModel) -> Vec<ClickArea> {
            self.calls.push("paint_confirm_dialog");
            Vec::new()
        }

        fn paint_budget_window(&mut self, _budget: &BudgetViewModel, _ingame: &InGameScreen) {
            self.calls.push("paint_budget_window");
        }

        fn paint_statistics_window(
            &mut self,
            _stats: &StatisticsWindowViewModel,
            _ingame: &InGameScreen,
        ) {
            self.calls.push("paint_statistics_window");
        }

        fn paint_inspect_window(
            &mut self,
            _inspect_pos: Option<(usize, usize)>,
            _map: &Map,
            _sim: &SimState,
            _ingame: &InGameScreen,
        ) {
            self.calls.push("paint_inspect_window");
        }

        fn paint_text_window(
            &mut self,
            _window_id: WindowId,
            _view: &TextWindowViewModel,
            _ingame: &mut InGameScreen,
        ) {
            self.calls.push("paint_text_window");
        }

        fn paint_advisor_window(&mut self, _advisor: &AdvisorViewModel, _ingame: &InGameScreen) {
            self.calls.push("paint_advisor_window");
        }

        fn paint_newspaper_window(
            &mut self,
            _newspaper: &NewspaperViewModel,
            _ingame: &InGameScreen,
        ) -> Vec<ClickArea> {
            self.calls.push("paint_newspaper_window");
            Vec::new()
        }

        fn paint_news_ticker(&mut self, _ticker: &NewsTickerViewModel) {
            self.calls.push("paint_news_ticker");
        }

        fn end_frame(&mut self) {
            self.calls.push("end_frame");
        }
    }

    /// Compile-time exhaustiveness check.
    /// If a field is added to `InGameDesktopView`, this destructure fails,
    /// forcing you to update `orchestrate_ingame()` and all painter implementations.
    #[test]
    fn view_model_fields_are_all_consumed_by_orchestrator() {
        let _check = |v: &InGameDesktopView| {
            let InGameDesktopView {
                map: _,
                sim: _,
                camera: _,
                current_tool: _,
                toolbar: _,
                tool_chooser: _,
                confirm_dialog: _,
                paused: _,
                overlay_mode: _,
                view_layer: _,
                menu_active: _,
                menu_selected: _,
                menu_item_selected: _,
                status_message: _,
                news_ticker: _,
                line_preview: _,
                rect_preview: _,
                inspect_pos: _,
                budget: _,
                statistics: _,
                help: _,
                about: _,
                legend: _,
                advisor: _,
                newspaper: _,
            } = v;
        };
    }

    #[test]
    fn orchestrate_ingame_calls_core_painter_methods_in_order() {
        use crate::{
            app::screens::BudgetFocus,
            core::{map::Map, sim::TaxRates},
            ui::{
                runtime::DesktopLayout,
                theme::OverlayMode,
                view::{BudgetViewModel, NewsTickerViewModel, ToolbarPaletteViewModel},
            },
        };

        let mut painter = MockPainter::default();
        let mut ingame = InGameScreen::new();
        let sim = crate::core::sim::SimState::default();
        let view = InGameDesktopView {
            map: Map::new(8, 8),
            sim: sim.clone(),
            camera: Camera::default(),
            current_tool: Tool::Inspect,
            toolbar: ToolbarPaletteViewModel {
                current_tool: Tool::Inspect,
                zone_tool: Tool::ZoneResLight,
                transport_tool: Tool::Road,
                utility_tool: Tool::PowerLine,
                power_plant_tool: Tool::PowerPlantCoal,
                building_tool: Tool::Police,
                terrain_tool: Tool::TerrainWater,
                chooser: None,
                view_layer: ViewLayer::Surface,
            },
            tool_chooser: None,
            confirm_dialog: None,
            paused: false,
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Surface,
            menu_active: false,
            menu_selected: 0,
            menu_item_selected: 0,
            status_message: Some("Ready".to_string()),
            news_ticker: NewsTickerViewModel::default(),
            line_preview: Vec::new(),
            rect_preview: Vec::new(),
            inspect_pos: None,
            budget: BudgetViewModel::from_sim(
                &sim,
                BudgetFocus::Residential,
                TaxRates::default(),
                "9".to_string(),
                "9".to_string(),
                "9".to_string(),
            ),
            statistics: None,
            help: None,
            about: None,
            legend: None,
            advisor: None,
            newspaper: None,
        };
        let layout = FrameLayout {
            desktop_layout: DesktopLayout::default(),
            view_w: 20,
            view_h: 15,
            col_scale: 2,
        };

        orchestrate_ingame(&mut painter, &view, &mut ingame, layout);

        assert_eq!(
            painter.calls,
            vec![
                "begin_frame",
                "paint_map",
                "paint_menu_bar",
                "paint_status_bar",
                "paint_panel_window",
                "paint_news_ticker",
                "end_frame",
            ]
        );
    }
}
