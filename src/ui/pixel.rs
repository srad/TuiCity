#![allow(dead_code)]

use std::{
    io,
    time::{Duration, Instant},
};

use font8x8::{UnicodeFonts, BASIC_FONTS, BLOCK_FONTS, BOX_FONTS, LATIN_FONTS, MISC_FONTS};
use miniquad::*;
use ratatui::style::Color;

use crate::{
    app::{
        config::FrontendKind,
        input::{self, Action, UiEvent},
        screens::{
            AppContext, InGameScreen, LlmSetupScreen, LoadCityScreen, NewCityScreen,
            SettingsScreen, StartScreen, ThemeSettingsScreen,
        },
        AppState, ClickArea,
    },
    core::{map::{Map, Tile, ViewLayer}, tool::Tool},
    ui::{
        runtime::{ToolChooserKind, ToolbarHitArea, ToolbarHitTarget, UiRect, WindowId},
        theme::{self, OverlayMode},
        view::{
            ConfirmDialogViewModel, InGameDesktopView, LlmSetupViewModel, LoadCityViewModel,
            NewCityViewModel, SettingsViewModel, StartViewModel, ThemeSettingsViewModel,
        },
    },
};

const DEFAULT_WINDOW_WIDTH: i32 = 1280;
const DEFAULT_WINDOW_HEIGHT: i32 = 800;
const CELL_WIDTH: u16 = 8;
const CELL_HEIGHT: u16 = 16;
const MIN_COLS: u16 = 80;
const MIN_ROWS: u16 = 30;
const PIXEL_TICK_MS: u64 = 33;

#[repr(C)]
struct Vertex {
    pos: (f32, f32),
    uv: (f32, f32),
}

struct PixelStage {
    app: AppState,
    framebuffer: Vec<u8>,
    cols: u16,
    rows: u16,
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    left_down: bool,
    middle_down: bool,
    needs_redraw: bool,
    last_tick: Instant,
    tick_interval: Duration,
}

#[derive(Clone)]
struct PixelMapSnapshot {
    minimap: ClickArea,
    viewport: ClickArea,
    offset_x: usize,
    offset_y: usize,
    view_w: usize,
    view_h: usize,
    overlay_mode: OverlayMode,
    view_layer: ViewLayer,
}

#[derive(Clone, Copy)]
struct ConnectionMask {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

#[derive(Clone, Copy)]
struct SquareGridLayout {
    x: usize,
    y: usize,
    tile_size: usize,
    grid_w: usize,
    grid_h: usize,
}

#[derive(Clone, Copy)]
struct FootprintTileRole {
    dx: usize,
    dy: usize,
    w: usize,
    h: usize,
}

impl PixelStage {
    fn new(_frontend: FrontendKind) -> Self {
        let (cols, rows) = grid_size(window::screen_size().0, window::screen_size().1);
        let mut app = AppState::new();
        app.attach_engine_channel();

        let mut ctx = window::new_rendering_backend();
        let texture = ctx.new_texture_from_rgba8(
            cols.max(1) * CELL_WIDTH,
            rows.max(1) * CELL_HEIGHT,
            &vec![0; rgba_len(cols, rows)],
        );
        ctx.texture_set_filter(texture, FilterMode::Nearest, MipmapFilterMode::None);

        let vertices: [Vertex; 4] = [
            Vertex {
                pos: (-1.0, -1.0),
                uv: (0.0, 1.0),
            },
            Vertex {
                pos: (1.0, -1.0),
                uv: (1.0, 1.0),
            },
            Vertex {
                pos: (1.0, 1.0),
                uv: (1.0, 0.0),
            },
            Vertex {
                pos: (-1.0, 1.0),
                uv: (0.0, 0.0),
            },
        ];
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );
        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![texture],
        };

        let shader = ctx
            .new_shader(
                match ctx.info().backend {
                    Backend::OpenGl => ShaderSource::Glsl {
                        vertex: shader::VERTEX,
                        fragment: shader::FRAGMENT,
                    },
                    Backend::Metal => ShaderSource::Msl {
                        program: shader::METAL,
                    },
                },
                shader::meta(),
            )
            .expect("pixel frontend shader should compile");
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams::default(),
        );

        Self {
            app,
            framebuffer: vec![0; rgba_len(cols, rows)],
            cols,
            rows,
            ctx,
            pipeline,
            bindings,
            left_down: false,
            middle_down: false,
            needs_redraw: true,
            last_tick: Instant::now(),
            tick_interval: Duration::from_millis(PIXEL_TICK_MS),
        }
    }

    fn sync_grid(&mut self, width: f32, height: f32) {
        let (cols, rows) = grid_size(width, height);
        if cols == self.cols && rows == self.rows {
            return;
        }

        self.cols = cols;
        self.rows = rows;
        self.framebuffer.resize(rgba_len(cols, rows), 0);
        self.ctx.texture_resize(
            self.bindings.images[0],
            u32::from(cols) * u32::from(CELL_WIDTH),
            u32::from(rows) * u32::from(CELL_HEIGHT),
            Some(&self.framebuffer),
        );
        self.app.on_event(&UiEvent::Resize { cols, rows });
        self.needs_redraw = true;
    }

    fn dispatch_action(&mut self, action: Action) {
        if !matches!(action, Action::None) {
            self.app.on_action(action);
            self.needs_redraw = true;
        }
    }

    fn render_pixels(&mut self) {
        render_pixel_app(&mut self.app, &mut self.framebuffer, self.cols, self.rows);
        self.ctx
            .texture_update(self.bindings.images[0], &self.framebuffer);
        self.needs_redraw = false;
    }

    fn cell_at(&self, x: f32, y: f32) -> (u16, u16) {
        let (screen_w, screen_h) = window::screen_size();
        let col = ((x.max(0.0) / screen_w.max(1.0)) * f32::from(self.cols)).floor() as u16;
        let row = ((y.max(0.0) / screen_h.max(1.0)) * f32::from(self.rows)).floor() as u16;
        (
            col.min(self.cols.saturating_sub(1)),
            row.min(self.rows.saturating_sub(1)),
        )
    }

    fn minimap_click_action(&mut self, x: f32, y: f32) -> Option<Action> {
        let snapshot = capture_ingame_snapshot(&mut self.app)?;
        let engine = self.app.engine.clone();
        let engine = engine.read().unwrap();
        let layout = square_minimap_layout(snapshot.minimap, engine.map.width, engine.map.height)?;
        let (tile_x, tile_y) = square_layout_tile_at_pixel(
            layout,
            engine.map.width,
            engine.map.height,
            x.max(0.0) as usize,
            y.max(0.0) as usize,
        )?;

        Some(Action::MouseClick {
            col: synthetic_minimap_col(snapshot.minimap, engine.map.width, tile_x),
            row: synthetic_minimap_row(snapshot.minimap, engine.map.height, tile_y),
        })
    }

    fn map_click_action(&mut self, x: f32, y: f32) -> Option<Action> {
        let snapshot = capture_ingame_snapshot(&mut self.app)?;
        let layout = square_map_layout(snapshot.viewport, snapshot.view_w, snapshot.view_h)?;
        let (view_x, view_y) = square_layout_tile_at_pixel(
            layout,
            snapshot.view_w,
            snapshot.view_h,
            x.max(0.0) as usize,
            y.max(0.0) as usize,
        )?;
        Some(Action::MouseClick {
            col: snapshot.viewport.x + (view_x as u16).saturating_mul(2),
            row: snapshot.viewport.y + view_y as u16,
        })
    }
}

impl EventHandler for PixelStage {
    fn update(&mut self) {
        if self.last_tick.elapsed() >= self.tick_interval {
            self.app.on_tick();
            self.last_tick = Instant::now();
            self.needs_redraw = true;
        }
        if !self.app.running {
            window::order_quit();
        }
    }

    fn draw(&mut self) {
        self.sync_grid(window::screen_size().0, window::screen_size().1);
        if self.needs_redraw {
            self.render_pixels();
        }

        self.ctx.begin_default_pass(Default::default());
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.sync_grid(width, height);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let action = if self.middle_down {
            let (col, row) = self.cell_at(x, y);
            Action::MouseMiddleDrag { col, row }
        } else if self.left_down {
            if let Some(Action::MouseClick { col, row }) = self.map_click_action(x, y) {
                Action::MouseDrag { col, row }
            } else {
                let (col, row) = self.cell_at(x, y);
                Action::MouseDrag { col, row }
            }
        } else {
            let (col, row) = self.cell_at(x, y);
            Action::MouseMove { col, row }
        };
        self.dispatch_action(action);
    }

    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
        if x.abs() >= 0.5 {
            self.dispatch_action(Action::PanCamera(x.signum() as i32 * 3, 0));
        }
        if y.abs() >= 0.5 {
            self.dispatch_action(Action::PanCamera(0, -(y.signum() as i32) * 3));
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            if let Some(action) = self.minimap_click_action(x, y) {
                self.dispatch_action(action);
                return;
            }
            if let Some(action) = self.map_click_action(x, y) {
                self.left_down = true;
                self.dispatch_action(action);
                return;
            }
        }
        let (col, row) = self.cell_at(x, y);
        match button {
            MouseButton::Left => {
                self.left_down = true;
                self.dispatch_action(Action::MouseClick { col, row });
            }
            MouseButton::Middle => {
                self.middle_down = true;
                self.dispatch_action(Action::MouseMiddleDown { col, row });
            }
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        let (col, row) = self.cell_at(x, y);
        match button {
            MouseButton::Left => {
                let was_down = self.left_down;
                self.left_down = false;
                if was_down {
                    self.dispatch_action(Action::MouseUp { col, row });
                }
            }
            MouseButton::Middle => {
                let was_down = self.middle_down;
                self.middle_down = false;
                if was_down {
                    self.dispatch_action(Action::MouseMiddleUp);
                }
            }
            _ => {}
        }
    }

    fn char_event(&mut self, character: char, keymods: KeyMods, _repeat: bool) {
        self.dispatch_action(input::translate_miniquad_char(character, keymods));
    }

    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, _repeat: bool) {
        self.dispatch_action(input::translate_miniquad_key(keycode, keymods));
    }

    fn quit_requested_event(&mut self) {
        window::cancel_quit();
        self.dispatch_action(Action::Quit);
        if !self.app.running {
            window::order_quit();
        }
    }
}

pub fn run_pixel(frontend: FrontendKind) -> io::Result<()> {
    miniquad::start(
        conf::Conf {
            window_title: "TuiCity 2000".to_string(),
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            high_dpi: false,
            fullscreen: false,
            sample_count: 1,
            window_resizable: true,
            icon: None,
            platform: Default::default(),
        },
        move || Box::new(PixelStage::new(frontend)),
    );
    Ok(())
}

fn grid_size(width: f32, height: f32) -> (u16, u16) {
    let cols = (width.max(f32::from(CELL_WIDTH)) / f32::from(CELL_WIDTH)).floor() as u16;
    let rows = (height.max(f32::from(CELL_HEIGHT)) / f32::from(CELL_HEIGHT)).floor() as u16;
    (cols.max(MIN_COLS), rows.max(MIN_ROWS))
}

fn rgba_len(cols: u16, rows: u16) -> usize {
    usize::from(cols) * usize::from(CELL_WIDTH) * usize::from(rows) * usize::from(CELL_HEIGHT) * 4
}

fn render_pixel_app(app: &mut AppState, framebuffer: &mut [u8], cols: u16, rows: u16) {
    theme::set_render_style(theme::RenderStyle::PixelDos);
    let width = usize::from(cols) * usize::from(CELL_WIDTH);
    let height = usize::from(rows) * usize::from(CELL_HEIGHT);
    let ui = theme::ui_palette();
    fill_rect(
        framebuffer,
        width,
        0,
        0,
        width,
        height,
        color_to_rgba(ui.desktop_bg, (0, 0, 0, 255)),
    );

    let context = AppContext {
        engine: &app.engine,
        cmd_tx: &app.cmd_tx,
        textgen: &app.textgen,
    };

    let Some(screen) = app.screens.last_mut() else {
        return;
    };
    if let Some(screen) = screen.as_any_mut().downcast_mut::<StartScreen>() {
        let view = screen.view_model();
        render_pixel_start(framebuffer, width, cols, rows, &view, screen);
    } else if let Some(screen) = screen.as_any_mut().downcast_mut::<SettingsScreen>() {
        let view = screen.view_model(context);
        render_pixel_settings(framebuffer, width, cols, rows, &view, screen);
    } else if let Some(screen) = screen.as_any_mut().downcast_mut::<ThemeSettingsScreen>() {
        let view = screen.view_model();
        render_pixel_theme_settings(framebuffer, width, cols, rows, &view, screen);
    } else if let Some(screen) = screen.as_any_mut().downcast_mut::<LoadCityScreen>() {
        let view = screen.view_model();
        render_pixel_load_city(framebuffer, width, cols, rows, &view, screen);
    } else if let Some(screen) = screen.as_any_mut().downcast_mut::<LlmSetupScreen>() {
        let view = screen.view_model(context);
        render_pixel_llm_setup(framebuffer, width, cols, rows, &view, screen);
    } else if let Some(screen) = screen.as_any_mut().downcast_mut::<NewCityScreen>() {
        let view = screen.view_model();
        render_pixel_new_city(framebuffer, width, cols, rows, &view, screen);
    } else if let Some(screen) = screen.as_any_mut().downcast_mut::<InGameScreen>() {
        let engine = context.engine.read().unwrap();
        let view = screen.view_model(&engine.sim, &engine.map);
        render_pixel_ingame(framebuffer, width, cols, rows, &view, screen);
    }
}

fn render_pixel_start(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &StartViewModel,
    screen: &mut StartScreen,
) {
    let ui = theme::ui_palette();
    let height = usize::from(rows) * usize::from(CELL_HEIGHT);
    screen.state.menu_areas = [ClickArea::default(); 5];
    draw_start_background(framebuffer, width, height);
    let panel_w = 42;
    let panel_h = 15;
    let panel_x = cols.saturating_sub(panel_w) / 2;
    let panel_y = rows.saturating_sub(panel_h) / 2 + 1;
    draw_box_cells(
        framebuffer,
        width,
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        Some(" MAIN MENU "),
        ui.window_border,
        ui.window_bg,
    );
    draw_centered_text_cells_scaled(
        framebuffer,
        width,
        cols,
        panel_y.saturating_sub(7),
        "TUICITY 2000",
        2,
        ui.title,
        Color::Reset,
    );
    draw_centered_text_cells(
        framebuffer,
        width,
        cols,
        panel_y.saturating_sub(2),
        "Retro city-builder frontend",
        ui.subtitle,
        Color::Reset,
    );
    draw_centered_text_cells(
        framebuffer,
        width,
        cols,
        panel_y.saturating_sub(1),
        "Mouse + keyboard supported",
        ui.text_dim,
        Color::Reset,
    );

    for (idx, option) in view.options.iter().enumerate() {
        let area = ClickArea {
            x: panel_x + 4,
            y: panel_y + 3 + idx as u16 * 2,
            width: panel_w.saturating_sub(8),
            height: 2,
        };
        screen.state.menu_areas[idx] = area;
        let selected = idx == view.selected;
        draw_button_row(
            framebuffer,
            width,
            area,
            option,
            selected,
            ui.selection_bg,
            ui.selection_fg,
            ui.button_bg,
            ui.button_fg,
        );
    }
}

fn render_pixel_settings(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &SettingsViewModel,
    screen: &mut SettingsScreen,
) {
    let ui = theme::ui_palette();
    let items: Vec<String> = view
        .options
        .iter()
        .cloned()
        .chain(std::iter::once("Back".to_string()))
        .collect();
    render_centered_list_panel(
        framebuffer,
        width,
        cols,
        rows,
        " SETTINGS ",
        &items,
        view.selected,
        &mut screen.state.row_areas,
        ui,
    );
    draw_centered_text_cells(
        framebuffer,
        width,
        cols,
        rows.saturating_sub(4),
        &format!("Theme: {}", view.current_theme_label),
        ui.text_dim,
        Color::Reset,
    );
}

fn render_pixel_theme_settings(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &ThemeSettingsViewModel,
    screen: &mut ThemeSettingsScreen,
) {
    let ui = theme::ui_palette();
    let items: Vec<String> = view
        .themes
        .iter()
        .map(|theme| {
            let active = if *theme == view.active { " *" } else { "" };
            format!("{}{}", theme.label(), active)
        })
        .chain(std::iter::once("Back".to_string()))
        .collect();
    render_centered_list_panel(
        framebuffer,
        width,
        cols,
        rows,
        " THEME SETTINGS ",
        &items,
        view.selected,
        &mut screen.state.row_areas,
        ui,
    );
}

fn render_pixel_load_city(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &LoadCityViewModel,
    screen: &mut LoadCityScreen,
) {
    let ui = theme::ui_palette();
    let mut items: Vec<String> = if view.is_loading {
        vec![format!("Scanning saves {}", view.loading_indicator)]
    } else if view.saves.is_empty() {
        vec!["No saves found".to_string()]
    } else {
        view.saves
            .iter()
            .map(|save| {
                format!(
                    "{}  {:02}/{}  Pop {}",
                    save.city_name, save.month, save.year, save.population
                )
            })
            .collect()
    };
    items.push("Back".to_string());
    render_centered_list_panel(
        framebuffer,
        width,
        cols,
        rows,
        " LOAD CITY ",
        &items,
        view.selected,
        &mut screen.state.row_areas,
        ui,
    );

    screen.state.dialog_items.clear();
    if let Some(dialog) = &view.confirm_dialog {
        screen.state.dialog_items =
            render_confirm_dialog_cells(framebuffer, width, cols, rows, dialog, ui);
    }
}

fn render_pixel_llm_setup(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &LlmSetupViewModel,
    screen: &mut LlmSetupScreen,
) {
    let ui = theme::ui_palette();
    let installed = if view.model_installed { "Installed" } else { "Not installed" };
    let enabled = if view.llm_enabled { "Enabled" } else { "Disabled" };
    let download = if let Some(progress) = &view.download_progress {
        let pct = progress.percent.map(|p| format!(" {p}%")).unwrap_or_default();
        format!("Download Model [{}{}]", progress.label, pct)
    } else {
        "Download Model".to_string()
    };
    let items = vec![
        format!("LLM: {enabled}"),
        format!("Model: {}", view.selected_model_label),
        format!("Execution: {}", view.gpu_mode_label),
        download,
        "Cancel Download".to_string(),
        format!("Delete Model ({installed})"),
        "Back".to_string(),
    ];
    render_centered_list_panel(
        framebuffer,
        width,
        cols,
        rows,
        " LLM SETUP ",
        &items,
        screen.state.selected,
        &mut screen.state.row_areas,
        ui,
    );
    draw_centered_text_cells(
        framebuffer,
        width,
        cols,
        rows.saturating_sub(5),
        &fit_text(&view.backend_status, cols.saturating_sub(4) as usize),
        ui.text_dim,
        Color::Reset,
    );
    if let Some(dialog) = &view.confirm_dialog {
        let _ = render_confirm_dialog_cells(framebuffer, width, cols, rows, dialog, ui);
    }
}

fn render_pixel_new_city(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &NewCityViewModel,
    screen: &mut NewCityScreen,
) {
    let ui = theme::ui_palette();
    let height = usize::from(rows) * usize::from(CELL_HEIGHT);
    draw_start_background(framebuffer, width, height);
    draw_centered_text_cells_scaled(
        framebuffer,
        width,
        cols,
        1,
        "FOUND NEW CITY",
        2,
        ui.title,
        Color::Reset,
    );
    draw_centered_text_cells(
        framebuffer,
        width,
        cols,
        5,
        "Shape the terrain, tune the seed, then launch your mayoral run",
        ui.subtitle,
        Color::Reset,
    );
    let left_w = 42;
    let form_x = 3;
    let form_y = 7;
    let map_x = left_w + 5;
    let map_y = 8;
    let map_w = cols.saturating_sub(map_x + 3).max(20);
    let map_h = rows.saturating_sub(map_y + 8).max(12);

    screen.state.field_areas = [ClickArea::default(); 8];
    screen.state.brush_areas = [ClickArea::default(); 4];
    screen.state.inner_map_area = ClickArea {
        x: map_x + 1,
        y: map_y + 1,
        width: map_w.saturating_sub(2),
        height: map_h.saturating_sub(2),
    };

    draw_box_cells(
        framebuffer,
        width,
        form_x,
        form_y,
        left_w,
        rows.saturating_sub(6),
        Some(" NEW CITY "),
        ui.window_border,
        ui.window_bg,
    );
    draw_text_cells(
        framebuffer,
        width,
        form_x + 2,
        form_y + 1,
        "Configure identity and world generation",
        ui.text_dim,
        Color::Reset,
    );
    draw_box_cells(
        framebuffer,
        width,
        map_x,
        map_y,
        map_w,
        map_h,
        Some(" MAP PREVIEW "),
        ui.window_border,
        ui.map_window_bg,
    );
    draw_text_cells(
        framebuffer,
        width,
        map_x + 2,
        map_y + 1,
        "Preview terrain and paint with the brushes below",
        ui.text_dim,
        Color::Reset,
    );

    let labels = [
        ("City Name", view.city_name.clone()),
        ("Generate Name", if view.llm_name_pending { "Working...".to_string() } else { "Generate".to_string() }),
        ("Seed", view.seed_text.clone()),
        ("Water", format!("{}%", view.water_pct)),
        ("Trees", format!("{}%", view.trees_pct)),
        ("Regenerate", "Regenerate".to_string()),
        ("Start", "Start".to_string()),
        ("Back", "Back".to_string()),
    ];
    for (idx, (label, value)) in labels.iter().enumerate() {
        let y = form_y + 3 + idx as u16 * 2;
        let area = ClickArea {
            x: form_x + 2,
            y,
            width: left_w.saturating_sub(4),
            height: 2,
        };
        screen.state.field_areas[idx] = area;
        let selected = idx == view.focused_field as usize;
        draw_button_row(
            framebuffer,
            width,
            area,
            &format!("{label}: {value}"),
            selected,
            ui.selection_bg,
            ui.selection_fg,
            ui.button_bg,
            ui.button_fg,
        );
    }

    let brush_labels = ["None", "Water", "Land", "Trees"];
    for (idx, label) in brush_labels.iter().enumerate() {
        let area = ClickArea {
            x: map_x + 1 + idx as u16 * 7,
            y: map_y + map_h + 1,
            width: 6,
            height: 2,
        };
        screen.state.brush_areas[idx] = area;
        let selected = match (idx, view.terrain_brush) {
            (0, None) => true,
            (1, Some(crate::app::screens::TerrainBrush::Water)) => true,
            (2, Some(crate::app::screens::TerrainBrush::Land)) => true,
            (3, Some(crate::app::screens::TerrainBrush::Trees)) => true,
            _ => false,
        };
        draw_button_row(
            framebuffer,
            width,
            area,
            label,
            selected,
            ui.selection_bg,
            ui.selection_fg,
            ui.button_bg,
            ui.button_fg,
        );
    }

    draw_square_map(
        framebuffer,
        width,
        screen.state.inner_map_area,
        &view.preview_map,
        0,
        0,
        view.preview_map.width.min(screen.state.inner_map_area.width as usize),
        view.preview_map.height.min(screen.state.inner_map_area.height as usize),
        Some(view.cursor),
        None,
        ViewLayer::Surface,
        OverlayMode::None,
    );
    draw_text_cells(
        framebuffer,
        width,
        map_x + 1,
        map_y + map_h + 4,
        &fit_text(
            if view.map_cursor_active {
                "Map cursor active  Arrows move cursor  Enter paints"
            } else {
                "Tab between fields or click anywhere to edit quickly"
            },
            map_w.saturating_sub(2) as usize,
        ),
        ui.text_secondary,
        Color::Reset,
    );
}

fn render_pixel_ingame(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    view: &InGameDesktopView,
    screen: &mut InGameScreen,
) {
    let ui = theme::ui_palette();
    let desktop_layout = screen.desktop.layout(UiRect::new(0, 0, cols, rows));
    screen.ui_areas = Default::default();
    screen.ui_areas.desktop = desktop_layout.clone();
    screen.camera.col_scale = 2;

    render_ingame_menu_bar(framebuffer, width, cols, screen, ui);
    render_ingame_status_bar(
        framebuffer,
        width,
        cols,
        screen,
        &view.sim,
        view.status_message.as_deref(),
        ui,
    );

    let map_win = desktop_layout.window(WindowId::Map);
    let panel_win = desktop_layout.window(WindowId::Panel);
    draw_window_box(framebuffer, width, map_win.outer, " MAP ", ui);
    draw_window_box(framebuffer, width, panel_win.outer, " TOOLBOX ", ui);

    let map_inner = map_win.inner;
    let viewport_w = map_inner.width.saturating_sub(1).max(8);
    let viewport_h = map_inner.height.saturating_sub(1).max(6);
    screen.camera.view_w = (viewport_w as usize / screen.camera.col_scale as usize).max(4);
    screen.camera.view_h = viewport_h as usize;
    screen.ui_areas.map.viewport = ClickArea {
        x: map_inner.x,
        y: map_inner.y,
        width: viewport_w,
        height: viewport_h,
    };
    screen.ui_areas.map.vertical_bar = ClickArea {
        x: map_inner.x + viewport_w,
        y: map_inner.y,
        width: 1,
        height: viewport_h,
    };
    screen.ui_areas.map.vertical_track = screen.ui_areas.map.vertical_bar;
    screen.ui_areas.map.vertical_dec = ClickArea {
        x: map_inner.x + viewport_w,
        y: map_inner.y,
        width: 1,
        height: 1,
    };
    screen.ui_areas.map.vertical_inc = ClickArea {
        x: map_inner.x + viewport_w,
        y: map_inner.y + viewport_h.saturating_sub(1),
        width: 1,
        height: 1,
    };
    screen.ui_areas.map.vertical_thumb = screen.ui_areas.map.vertical_bar;
    screen.ui_areas.map.horizontal_bar = ClickArea {
        x: map_inner.x,
        y: map_inner.y + viewport_h,
        width: viewport_w,
        height: 1,
    };
    screen.ui_areas.map.horizontal_track = screen.ui_areas.map.horizontal_bar;
    screen.ui_areas.map.horizontal_dec = ClickArea {
        x: map_inner.x,
        y: map_inner.y + viewport_h,
        width: 1,
        height: 1,
    };
    screen.ui_areas.map.horizontal_inc = ClickArea {
        x: map_inner.x + viewport_w.saturating_sub(1),
        y: map_inner.y + viewport_h,
        width: 1,
        height: 1,
    };
    screen.ui_areas.map.horizontal_thumb = screen.ui_areas.map.horizontal_bar;
    screen.ui_areas.map.corner = ClickArea {
        x: map_inner.x + viewport_w,
        y: map_inner.y + viewport_h,
        width: 1,
        height: 1,
    };

    render_ingame_panel(framebuffer, width, panel_win.inner, view, screen, ui);
    draw_square_map(
        framebuffer,
        width,
        screen.ui_areas.map.viewport,
        &view.map,
        screen.camera.offset_x.max(0) as usize,
        screen.camera.offset_y.max(0) as usize,
        screen.camera.view_w.max(1),
        screen.camera.view_h.max(1),
        Some((screen.camera.cursor_x, screen.camera.cursor_y)),
        screen.inspect_pos,
        screen.view_layer,
        screen.overlay_mode,
    );

    render_optional_ingame_windows(framebuffer, width, view, screen, ui);
    if let Some(dialog) = &view.confirm_dialog {
        screen.ui_areas.dialog_items =
            render_confirm_dialog_cells(framebuffer, width, cols, rows, dialog, ui);
    }

    render_news_ticker(framebuffer, width, cols, rows, &view.news_ticker.full_text, ui);
}

fn render_centered_list_panel(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    title: &str,
    items: &[String],
    selected: usize,
    areas: &mut Vec<ClickArea>,
    ui: theme::UiPalette,
) {
    let height = usize::from(rows) * usize::from(CELL_HEIGHT);
    draw_start_background(framebuffer, width, height);
    areas.clear();
    let panel_w = 72.min(cols.saturating_sub(6)).max(28);
    let panel_h = (items.len() as u16 * 2 + 7)
        .min(rows.saturating_sub(6))
        .max(10);
    let panel_x = cols.saturating_sub(panel_w) / 2;
    let panel_y = rows.saturating_sub(panel_h) / 2;
    draw_box_cells(
        framebuffer,
        width,
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        Some(title),
        ui.window_border,
        ui.window_bg,
    );
    draw_centered_text_cells(
        framebuffer,
        width,
        cols,
        panel_y.saturating_sub(2),
        "Navigate with arrows or click directly",
        ui.subtitle,
        Color::Reset,
    );
    draw_text_cells(
        framebuffer,
        width,
        panel_x + 2,
        panel_y + 1,
        "CITY OPERATIONS",
        ui.text_dim,
        Color::Reset,
    );
    for (idx, item) in items.iter().enumerate() {
        let y = panel_y + 3 + idx as u16 * 2;
        if y + 1 >= panel_y + panel_h - 1 {
            break;
        }
        let area = ClickArea {
            x: panel_x + 2,
            y,
            width: panel_w.saturating_sub(4),
            height: 2,
        };
        areas.push(area);
        draw_button_row(
            framebuffer,
            width,
            area,
            item,
            idx == selected,
            ui.selection_bg,
            ui.selection_fg,
            ui.button_bg,
            ui.button_fg,
        );
    }
    draw_text_cells(
        framebuffer,
        width,
        panel_x + 2,
        panel_y + panel_h.saturating_sub(2),
        &fit_text("Enter confirms  Esc / Back returns", panel_w.saturating_sub(4) as usize),
        ui.text_secondary,
        Color::Reset,
    );
}

fn render_confirm_dialog_cells(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    dialog: &ConfirmDialogViewModel,
    ui: theme::UiPalette,
) -> Vec<ClickArea> {
    shade_pixels(framebuffer, (0, 0, 0, 255), 140);
    let panel_w = 56.min(cols.saturating_sub(8)).max(24);
    let panel_h = 11.min(rows.saturating_sub(6)).max(8);
    let panel_x = cols.saturating_sub(panel_w) / 2;
    let panel_y = rows.saturating_sub(panel_h) / 2;
    draw_box_cells(
        framebuffer,
        width,
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        Some(&format!(" {} ", dialog.title)),
        ui.popup_border,
        ui.popup_bg,
    );
    draw_text_cells(
        framebuffer,
        width,
        panel_x + 2,
        panel_y + 1,
        "CONFIRM ACTION",
        ui.text_dim,
        Color::Reset,
    );
    for (idx, line) in wrap_text_lines(&dialog.message, panel_w.saturating_sub(4) as usize, 3)
        .into_iter()
        .enumerate()
    {
        draw_text_cells(
            framebuffer,
            width,
            panel_x + 2,
            panel_y + 3 + idx as u16,
            &line,
            ui.text_primary,
            Color::Reset,
        );
    }
    let total_w: u16 = dialog
        .buttons
        .iter()
        .map(|button| button.label.len() as u16 + 6)
        .sum::<u16>()
        + dialog.buttons.len().saturating_sub(1) as u16;
    let mut x = panel_x + panel_w.saturating_sub(total_w) / 2;
    let y = panel_y + panel_h - 3;
    let mut hits = Vec::new();
    for (idx, button) in dialog.buttons.iter().enumerate() {
        let w = button.label.len() as u16 + 6;
        let area = ClickArea {
            x,
            y,
            width: w,
            height: 2,
        };
        hits.push(area);
        draw_button_row(
            framebuffer,
            width,
            area,
            &format!(" {} ", button.label),
            idx == dialog.selected,
            ui.selection_bg,
            ui.selection_fg,
            ui.button_bg,
            ui.button_fg,
        );
        x += w + 1;
    }
    hits
}

fn render_ingame_menu_bar(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    screen: &mut InGameScreen,
    ui: theme::UiPalette,
) {
    fill_cell_rect(
        framebuffer,
        width,
        0,
        0,
        cols,
        1,
        color_to_rgba(ui.menu_bg, (0, 0, 0, 255)),
    );
    screen.ui_areas.menu_bar = ClickArea { x: 0, y: 0, width: cols, height: 1 };
    let mut x = 1;
    let title = " TuiCity 2000 ";
    draw_text_cells(framebuffer, width, x, 0, title, ui.menu_title, ui.menu_bg);
    x += title.len() as u16 + 1;
    for (idx, label) in crate::app::screens::MENU_TITLES.iter().enumerate() {
        let text = format!(" {} ", label);
        if x + text.len() as u16 >= cols {
            break;
        }
        screen.ui_areas.menu_items[idx] = ClickArea { x, y: 0, width: text.len() as u16, height: 1 };
        draw_button_row(
            framebuffer,
            width,
            screen.ui_areas.menu_items[idx],
            &text,
            screen.menu_active && screen.menu_selected == idx,
            ui.menu_focus_bg,
            ui.menu_focus_fg,
            ui.menu_bg,
            ui.menu_fg,
        );
        x += text.len() as u16 + 1;
    }
    if screen.menu_active {
        let rows = crate::app::screens::menu_rows(screen.menu_selected);
        screen.ui_areas.menu_popup_items.clear();
        if let Some(anchor) = screen.ui_areas.menu_items.get(screen.menu_selected).copied() {
            let popup_x = anchor.x;
            let popup_y = 1;
            let desired_w = rows
                .iter()
                .map(|row| row.label.len() + row.right.len() + 6)
                .max()
                .unwrap_or(12) as u16;
            let popup_w = desired_w.min(cols.saturating_sub(popup_x + 1)).max(18);
            let popup_h = rows.len() as u16 * 2 + 2;
            screen.ui_areas.menu_popup = ClickArea { x: popup_x, y: popup_y, width: popup_w, height: popup_h };
            draw_box_cells(
                framebuffer,
                width,
                popup_x,
                popup_y,
                popup_w,
                popup_h,
                None,
                ui.window_border,
                ui.window_bg,
            );
            for (idx, row) in rows.iter().enumerate() {
                let area = ClickArea {
                    x: popup_x + 1,
                    y: popup_y + 1 + idx as u16 * 2,
                    width: popup_w.saturating_sub(2),
                    height: 2,
                };
                screen.ui_areas.menu_popup_items.push(area);
                draw_button_row(
                    framebuffer,
                    width,
                    area,
                    &format_menu_row_text(row.label, row.right, area.width as usize),
                    idx == screen.menu_item_selected,
                    ui.selection_bg,
                    ui.selection_fg,
                    ui.window_bg,
                    ui.text_primary,
                );
            }
        }
    }
}

fn render_ingame_status_bar(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    screen: &mut InGameScreen,
    sim: &crate::core::sim::SimState,
    status_message: Option<&str>,
    ui: theme::UiPalette,
) {
    fill_cell_rect(
        framebuffer,
        width,
        0,
        1,
        cols,
        1,
        color_to_rgba(ui.status_bg, (0, 0, 0, 255)),
    );
    let left = format!(
        " {}  ${}  Pop {}  {}/{} ",
        sim.city_name, sim.economy.treasury, sim.pop.population, sim.month, sim.year
    );
    draw_text_cells(framebuffer, width, 0, 1, &fit_text(&left, cols as usize), ui.status_city, ui.status_bg);
    if let Some(message) = status_message {
        let clipped = fit_text(message, cols.saturating_sub(40) as usize);
        if !clipped.is_empty() {
            let start = ((cols / 2).saturating_sub((clipped.len() as u16) / 2)).max(24);
            draw_text_cells(framebuffer, width, start, 1, &clipped, ui.status_message, ui.status_bg);
        }
    }
    let pause_text = if screen.paused { " Run " } else { " Pause " };
    let pause_w = pause_text.len() as u16;
    let layer_surface = " Surf ";
    let layer_underground = " Undr ";
    let right_start = cols.saturating_sub(pause_w + layer_surface.len() as u16 + layer_underground.len() as u16 + 3);
    screen.ui_areas.layer_surface_btn = ClickArea { x: right_start, y: 1, width: layer_surface.len() as u16, height: 1 };
    screen.ui_areas.layer_underground_btn = ClickArea { x: right_start + layer_surface.len() as u16 + 1, y: 1, width: layer_underground.len() as u16, height: 1 };
    screen.ui_areas.pause_btn = ClickArea { x: cols.saturating_sub(pause_w), y: 1, width: pause_w, height: 1 };
    draw_button_row(framebuffer, width, screen.ui_areas.layer_surface_btn, layer_surface, screen.view_layer == ViewLayer::Surface, ui.selection_bg, ui.selection_fg, ui.button_bg, ui.button_fg);
    draw_button_row(framebuffer, width, screen.ui_areas.layer_underground_btn, layer_underground, screen.view_layer == ViewLayer::Underground, ui.selection_bg, ui.selection_fg, ui.button_bg, ui.button_fg);
    draw_button_row(framebuffer, width, screen.ui_areas.pause_btn, pause_text, screen.paused, ui.selection_bg, ui.selection_fg, ui.button_bg, ui.button_fg);
}

fn render_ingame_panel(
    framebuffer: &mut [u8],
    width: usize,
    inner: crate::ui::runtime::UiRect,
    view: &InGameDesktopView,
    screen: &mut InGameScreen,
    ui: theme::UiPalette,
) {
    screen.ui_areas.tool_chooser_items.clear();
    let mut toolbar_items = Vec::new();
    let active_area = ClickArea {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: 5,
    };
    draw_box_cells(
        framebuffer,
        width,
        active_area.x,
        active_area.y,
        active_area.width,
        active_area.height,
        Some(" FOCUS "),
        ui.window_border,
        ui.panel_window_bg,
    );
    draw_text_cells(
        framebuffer,
        width,
        active_area.x + 1,
        active_area.y + 1,
        "ACTIVE TOOL",
        ui.toolbar_header,
        Color::Reset,
    );
    draw_text_cells_scaled(
        framebuffer,
        width,
        active_area.x + 1,
        active_area.y + 2,
        &fit_text(
            view.current_tool.label(),
            active_area.width.saturating_sub(3) as usize / 2,
        ),
        2,
        ui.window_title,
        Color::Reset,
    );
    let meta_row = active_area.y + active_area.height.saturating_sub(1);
    let layer_chip_width = active_area.width.saturating_sub(3) / 2;
    let layer_chip = ClickArea {
        x: active_area.x + 1,
        y: meta_row,
        width: layer_chip_width.max(8),
        height: 1,
    };
    let overlay_chip = ClickArea {
        x: layer_chip.x + layer_chip.width + 1,
        y: meta_row,
        width: active_area
            .width
            .saturating_sub(layer_chip.width)
            .saturating_sub(3),
        height: 1,
    };
    draw_button_row(
        framebuffer,
        width,
        layer_chip,
        &fit_text(
            layer_status_text(view.view_layer),
            layer_chip.width.saturating_sub(2) as usize,
        ),
        false,
        ui.selection_bg,
        ui.selection_fg,
        ui.button_bg,
        ui.button_fg,
    );
    draw_button_row(
        framebuffer,
        width,
        overlay_chip,
        &fit_text(
            &format!("{}  ${}", overlay_status_text(view.overlay_mode), view.current_tool.cost()),
            overlay_chip.width.saturating_sub(2) as usize,
        ),
        view.overlay_mode != OverlayMode::None,
        ui.selection_bg,
        ui.selection_fg,
        ui.button_bg,
        ui.button_fg,
    );

    let toolbar_title_y = active_area.y + active_area.height + 1;
    draw_text_cells(
        framebuffer,
        width,
        inner.x + 1,
        toolbar_title_y,
        "BUILD PALETTE",
        ui.toolbar_header,
        Color::Reset,
    );
    let buttons = [
        (
            "Inspect",
            view.current_tool.label().to_string(),
            ToolbarHitTarget::SelectTool(Tool::Inspect),
            view.current_tool == Tool::Inspect,
        ),
        (
            "Bulldoze",
            format!("Clear  ${}", Tool::Bulldoze.cost()),
            ToolbarHitTarget::SelectTool(Tool::Bulldoze),
            view.current_tool == Tool::Bulldoze,
        ),
        (
            "Zones",
            view.toolbar.zone_tool.label().to_string(),
            ToolbarHitTarget::OpenChooser(ToolChooserKind::Zones),
            view.toolbar.chooser == Some(ToolChooserKind::Zones)
                || ToolChooserKind::for_tool(view.current_tool) == Some(ToolChooserKind::Zones),
        ),
        (
            "Transport",
            view.toolbar.transport_tool.label().to_string(),
            ToolbarHitTarget::OpenChooser(ToolChooserKind::Transport),
            view.toolbar.chooser == Some(ToolChooserKind::Transport)
                || ToolChooserKind::for_tool(view.current_tool) == Some(ToolChooserKind::Transport),
        ),
        (
            "Utilities",
            view.toolbar.utility_tool.label().to_string(),
            ToolbarHitTarget::OpenChooser(ToolChooserKind::Utilities),
            view.toolbar.chooser == Some(ToolChooserKind::Utilities)
                || ToolChooserKind::for_tool(view.current_tool) == Some(ToolChooserKind::Utilities),
        ),
        (
            "Plants",
            view.toolbar.power_plant_tool.label().to_string(),
            ToolbarHitTarget::OpenChooser(ToolChooserKind::PowerPlants),
            view.toolbar.chooser == Some(ToolChooserKind::PowerPlants)
                || ToolChooserKind::for_tool(view.current_tool) == Some(ToolChooserKind::PowerPlants),
        ),
        (
            "Buildings",
            view.toolbar.building_tool.label().to_string(),
            ToolbarHitTarget::OpenChooser(ToolChooserKind::Buildings),
            view.toolbar.chooser == Some(ToolChooserKind::Buildings)
                || ToolChooserKind::for_tool(view.current_tool) == Some(ToolChooserKind::Buildings),
        ),
        (
            "Terrain",
            view.toolbar.terrain_tool.label().to_string(),
            ToolbarHitTarget::OpenChooser(ToolChooserKind::Terrain),
            view.toolbar.chooser == Some(ToolChooserKind::Terrain)
                || ToolChooserKind::for_tool(view.current_tool) == Some(ToolChooserKind::Terrain),
        ),
    ];
    for (idx, (label, detail, target, selected)) in buttons.iter().enumerate() {
        let area = ClickArea {
            x: inner.x + 1,
            y: toolbar_title_y + 1 + idx as u16 * 2,
            width: inner.width.saturating_sub(2),
            height: 2,
        };
        draw_tool_button(
            framebuffer,
            width,
            area,
            label,
            detail,
            *selected,
            true,
            toolbar_target_accent(*target),
            ui,
        );
        toolbar_items.push(ToolbarHitArea { area, target: *target });
    }
    screen.ui_areas.toolbar_items = toolbar_items;

    let minimap_y = toolbar_title_y + 1 + buttons.len() as u16 * 2 + 2;
    let footer_y = inner.y + inner.height.saturating_sub(3);
    let minimap_h = footer_y.saturating_sub(minimap_y).max(6);
    screen.ui_areas.minimap = ClickArea {
        x: inner.x + 1,
        y: minimap_y,
        width: inner.width.saturating_sub(2),
        height: minimap_h,
    };
    draw_box_cells(
        framebuffer,
        width,
        screen.ui_areas.minimap.x.saturating_sub(1),
        screen.ui_areas.minimap.y.saturating_sub(1),
        screen.ui_areas.minimap.width + 2,
        screen.ui_areas.minimap.height + 2,
        Some(" MINIMAP "),
        ui.window_border,
        ui.panel_window_bg,
    );
    draw_square_minimap(
        framebuffer,
        width,
        screen.ui_areas.minimap,
        &view.map,
        screen.camera.offset_x.max(0) as usize,
        screen.camera.offset_y.max(0) as usize,
        screen.camera.view_w.max(1),
        screen.camera.view_h.max(1),
        screen.view_layer,
        screen.overlay_mode,
    );
    draw_text_cells(
        framebuffer,
        width,
        inner.x + 1,
        screen.ui_areas.minimap.y + screen.ui_areas.minimap.height + 1,
        &fit_text(
            &format!(
                "{}  Cam {},{}",
                layer_status_text(view.toolbar.view_layer),
                screen.camera.offset_x.max(0),
                screen.camera.offset_y.max(0)
            ),
            inner.width.saturating_sub(2) as usize,
        ),
        ui.text_secondary,
        Color::Reset,
    );
    draw_text_cells(
        framebuffer,
        width,
        inner.x + 1,
        screen.ui_areas.minimap.y + screen.ui_areas.minimap.height + 2,
        &fit_text(
            &format!("{}  Drag MMB to pan", overlay_status_text(view.overlay_mode)),
            inner.width.saturating_sub(2) as usize,
        ),
        ui.text_dim,
        Color::Reset,
    );
    if let Some(chooser) = &view.tool_chooser {
        screen.ui_areas.tool_chooser_items.clear();
        let popup = screen.ui_areas.desktop.window(WindowId::PowerPicker).outer;
        draw_window_box(framebuffer, width, popup, " TOOLS ", ui);
        for (idx, tool) in chooser.tools.iter().enumerate() {
            let detail = tool
                .unavailable_reason(&chooser.ctx)
                .unwrap_or_else(|| format!("Cost ${}", tool.cost()));
            let area = ClickArea {
                x: popup.x + 2,
                y: popup.y + 2 + idx as u16 * 2,
                width: popup.width.saturating_sub(4),
                height: 2,
            };
            screen.ui_areas.tool_chooser_items.push((area, *tool));
            draw_tool_button(
                framebuffer,
                width,
                area,
                tool.label(),
                &detail,
                *tool == chooser.selected_tool,
                tool.is_available(&chooser.ctx),
                toolbar_target_accent(ToolbarHitTarget::SelectTool(*tool)),
                ui,
            );
        }
    }
}

fn render_optional_ingame_windows(
    framebuffer: &mut [u8],
    width: usize,
    view: &InGameDesktopView,
    screen: &mut InGameScreen,
    ui: theme::UiPalette,
) {
    if screen.is_budget_open() {
        let win = screen.ui_areas.desktop.window(WindowId::Budget).outer;
        draw_window_box(framebuffer, width, win, " BUDGET ", ui);
        draw_text_cells(framebuffer, width, win.x + 2, win.y + 2, &format!("Residential Tax: {}%", view.budget.tax_rates.residential), ui.text_primary, ui.window_bg);
        draw_text_cells(framebuffer, width, win.x + 2, win.y + 3, &format!("Commercial Tax: {}%", view.budget.tax_rates.commercial), ui.text_primary, ui.window_bg);
        draw_text_cells(framebuffer, width, win.x + 2, win.y + 4, &format!("Industrial Tax: {}%", view.budget.tax_rates.industrial), ui.text_primary, ui.window_bg);
    }
    if let Some(stats) = &view.statistics {
        let win = screen.ui_areas.desktop.window(WindowId::Statistics).outer;
        draw_window_box(framebuffer, width, win, " STATS ", ui);
        draw_text_cells(framebuffer, width, win.x + 2, win.y + 2, &format!("Population: {}", stats.current_population), ui.text_primary, ui.window_bg);
        draw_text_cells(framebuffer, width, win.x + 2, win.y + 3, &format!("Treasury: ${}", stats.current_treasury), ui.text_primary, ui.window_bg);
    }
    if let Some(help) = &view.help {
        let win = screen.ui_areas.desktop.window(WindowId::Help).outer;
        draw_window_box(framebuffer, width, win, " HELP ", ui);
        for (idx, line) in help.lines.iter().take(win.height.saturating_sub(3) as usize).enumerate() {
            draw_text_cells(framebuffer, width, win.x + 2, win.y + 2 + idx as u16, &fit_text(line, win.width.saturating_sub(4) as usize), ui.text_primary, ui.window_bg);
        }
    }
    if let Some(about) = &view.about {
        let win = screen.ui_areas.desktop.window(WindowId::About).outer;
        draw_window_box(framebuffer, width, win, " ABOUT ", ui);
        for (idx, line) in about.lines.iter().take(win.height.saturating_sub(3) as usize).enumerate() {
            draw_text_cells(framebuffer, width, win.x + 2, win.y + 2 + idx as u16, &fit_text(line, win.width.saturating_sub(4) as usize), ui.text_primary, ui.window_bg);
        }
    }
    if let Some(legend) = &view.legend {
        let win = screen.ui_areas.desktop.window(WindowId::Legend).outer;
        draw_window_box(framebuffer, width, win, " LEGEND ", ui);
        for (idx, line) in legend.lines.iter().take(win.height.saturating_sub(3) as usize).enumerate() {
            draw_text_cells(framebuffer, width, win.x + 2, win.y + 2 + idx as u16, &fit_text(line, win.width.saturating_sub(4) as usize), ui.text_primary, ui.window_bg);
        }
    }
    if let Some(advisor) = &view.advisor {
        let win = screen.ui_areas.desktop.window(WindowId::Advisor).outer;
        draw_window_box(framebuffer, width, win, " ADVISOR ", ui);
        let text = advisor.text.as_deref().unwrap_or(if advisor.pending { "Thinking..." } else { "No advice yet." });
        draw_text_cells(framebuffer, width, win.x + 2, win.y + 2, &fit_text(text, win.width.saturating_sub(4) as usize), ui.text_primary, ui.window_bg);
    }
    if let Some(newspaper) = &view.newspaper {
        let win = screen.ui_areas.desktop.window(WindowId::Newspaper).outer;
        draw_window_box(framebuffer, width, win, " NEWSPAPER ", ui);
        screen.ui_areas.newspaper_sections.clear();
        if let Some(page) = newspaper.pages.get(newspaper.current_page) {
            draw_text_cells(framebuffer, width, win.x + 2, win.y + 2, &fit_text(&page.title, win.width.saturating_sub(4) as usize), ui.window_title, ui.window_bg);
            for (idx, section) in page.sections.iter().take(3).enumerate() {
                let area = ClickArea { x: win.x + 2, y: win.y + 4 + idx as u16 * 2, width: win.width.saturating_sub(4), height: 1 };
                screen.ui_areas.newspaper_sections.push(area);
                draw_button_row(framebuffer, width, area, &fit_text(&section.title, area.width as usize), idx == newspaper.selected_section_index, ui.selection_bg, ui.selection_fg, ui.window_bg, ui.text_primary);
            }
        }
    }
}

fn render_news_ticker(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    rows: u16,
    text: &str,
    ui: theme::UiPalette,
) {
    let row = rows.saturating_sub(1);
    fill_cell_rect(framebuffer, width, 0, row, cols, 1, color_to_rgba(ui.news_ticker_bg, (0, 0, 0, 255)));
    draw_text_cells(framebuffer, width, 0, row, &fit_text(text, cols as usize), ui.news_ticker_text, ui.news_ticker_bg);
}

fn draw_square_map(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    area: ClickArea,
    map: &Map,
    offset_x: usize,
    offset_y: usize,
    view_w: usize,
    view_h: usize,
    cursor: Option<(usize, usize)>,
    inspect: Option<(usize, usize)>,
    view_layer: ViewLayer,
    overlay_mode: OverlayMode,
) {
    let snapshot = PixelMapSnapshot {
        minimap: ClickArea::default(),
        viewport: area,
        offset_x,
        offset_y,
        view_w: view_w.max(1),
        view_h: view_h.max(1),
        overlay_mode,
        view_layer,
    };
    let area_x = usize::from(area.x) * usize::from(CELL_WIDTH);
    let area_y = usize::from(area.y) * usize::from(CELL_HEIGHT);
    let area_w = usize::from(area.width) * usize::from(CELL_WIDTH);
    let area_h = usize::from(area.height) * usize::from(CELL_HEIGHT);
    fill_rect(framebuffer, framebuffer_width, area_x, area_y, area_w, area_h, (8, 12, 18, 255));
    let Some(layout) = square_map_layout(area, snapshot.view_w, snapshot.view_h) else {
        return;
    };
    for vy in 0..snapshot.view_h {
        for vx in 0..snapshot.view_w {
            let map_x = snapshot.offset_x + vx;
            let map_y = snapshot.offset_y + vy;
            if map_x >= map.width || map_y >= map.height {
                continue;
            }
            draw_square_tile(
                framebuffer,
                framebuffer_width,
                map,
                &snapshot,
                map_x,
                map_y,
                layout.x + vx * layout.tile_size,
                layout.y + vy * layout.tile_size,
                layout.tile_size,
            );
        }
    }
    if let Some((cx, cy)) = cursor {
        if cx >= snapshot.offset_x
            && cx < snapshot.offset_x + snapshot.view_w
            && cy >= snapshot.offset_y
            && cy < snapshot.offset_y + snapshot.view_h
        {
            let vx = cx - snapshot.offset_x;
            let vy = cy - snapshot.offset_y;
            draw_rect_outline(
                framebuffer,
                framebuffer_width,
                layout.x + vx * layout.tile_size,
                layout.y + vy * layout.tile_size,
                layout.tile_size,
                layout.tile_size,
                (243, 208, 117, 255),
            );
        }
    }
    if let Some((ix, iy)) = inspect {
        if ix >= snapshot.offset_x
            && ix < snapshot.offset_x + snapshot.view_w
            && iy >= snapshot.offset_y
            && iy < snapshot.offset_y + snapshot.view_h
        {
            let vx = ix - snapshot.offset_x;
            let vy = iy - snapshot.offset_y;
            draw_rect_outline(
                framebuffer,
                framebuffer_width,
                layout.x + vx * layout.tile_size + 2,
                layout.y + vy * layout.tile_size + 2,
                layout.tile_size.saturating_sub(4),
                layout.tile_size.saturating_sub(4),
                (90, 210, 255, 255),
            );
        }
    }
}

fn draw_square_minimap(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    area: ClickArea,
    map: &Map,
    offset_x: usize,
    offset_y: usize,
    view_w: usize,
    view_h: usize,
    view_layer: ViewLayer,
    overlay_mode: OverlayMode,
) {
    let snapshot = PixelMapSnapshot {
        minimap: area,
        viewport: ClickArea::default(),
        offset_x,
        offset_y,
        view_w: view_w.max(1),
        view_h: view_h.max(1),
        overlay_mode,
        view_layer,
    };
    let Some(layout) = square_minimap_layout(area, map.width, map.height) else {
        return;
    };
    let area_x = usize::from(area.x) * usize::from(CELL_WIDTH);
    let area_y = usize::from(area.y) * usize::from(CELL_HEIGHT);
    let area_w = usize::from(area.width) * usize::from(CELL_WIDTH);
    let area_h = usize::from(area.height) * usize::from(CELL_HEIGHT);
    fill_rect(framebuffer, framebuffer_width, area_x, area_y, area_w, area_h, (8, 12, 18, 255));
    for gy in 0..layout.grid_h {
        for gx in 0..layout.grid_w {
            let map_x = if layout.grid_w <= 1 { 0 } else { gx * map.width.saturating_sub(1) / (layout.grid_w - 1) };
            let map_y = if layout.grid_h <= 1 { 0 } else { gy * map.height.saturating_sub(1) / (layout.grid_h - 1) };
            let tile = match snapshot.view_layer {
                ViewLayer::Surface => map.surface_lot_tile(map_x, map_y),
                ViewLayer::Underground => map.view_tile(snapshot.view_layer, map_x, map_y),
            };
            let overlay = map.get_overlay(map_x, map_y);
            let mut color = tile_fill_color(tile, snapshot.view_layer);
            if let Some(tint) = theme::overlay_tint(snapshot.overlay_mode, overlay) {
                color = blend_color(color, color_to_rgba(tint, color), 96);
            }
            fill_rect(
                framebuffer,
                framebuffer_width,
                layout.x + gx * layout.tile_size,
                layout.y + gy * layout.tile_size,
                layout.tile_size,
                layout.tile_size,
                color,
            );
        }
    }
    let vx0 = if map.width <= 1 || layout.grid_w <= 1 {
        0
    } else {
        snapshot.offset_x.min(map.width.saturating_sub(1)) * (layout.grid_w - 1) / (map.width - 1)
    };
    let vy0 = if map.height <= 1 || layout.grid_h <= 1 {
        0
    } else {
        snapshot.offset_y.min(map.height.saturating_sub(1)) * (layout.grid_h - 1) / (map.height - 1)
    };
    let vx1 = if map.width <= 1 || layout.grid_w <= 1 {
        layout.grid_w.saturating_sub(1)
    } else {
        (snapshot.offset_x + snapshot.view_w).min(map.width.saturating_sub(1)) * (layout.grid_w - 1) / (map.width - 1)
    };
    let vy1 = if map.height <= 1 || layout.grid_h <= 1 {
        layout.grid_h.saturating_sub(1)
    } else {
        (snapshot.offset_y + snapshot.view_h).min(map.height.saturating_sub(1)) * (layout.grid_h - 1) / (map.height - 1)
    };
    draw_rect_outline(
        framebuffer,
        framebuffer_width,
        layout.x + vx0 * layout.tile_size,
        layout.y + vy0 * layout.tile_size,
        (vx1.saturating_sub(vx0) + 1) * layout.tile_size,
        (vy1.saturating_sub(vy0) + 1) * layout.tile_size,
        (243, 208, 117, 255),
    );
    draw_rect_outline(
        framebuffer,
        framebuffer_width,
        layout.x,
        layout.y,
        layout.grid_w * layout.tile_size,
        layout.grid_h * layout.tile_size,
        (62, 78, 104, 255),
    );
}

fn draw_window_box(
    framebuffer: &mut [u8],
    width: usize,
    rect: UiRect,
    title: &str,
    ui: theme::UiPalette,
) {
    draw_box_cells(
        framebuffer,
        width,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        Some(title),
        ui.window_border,
        ui.window_bg,
    );
}

fn draw_start_background(framebuffer: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        let blend = ((y * 255) / height.max(1)) as u8;
        let color = blend_color((7, 12, 24, 255), (48, 32, 92, 255), blend);
        draw_hline(framebuffer, width, 0, width.saturating_sub(1), y, color);
    }

    let horizon = height.saturating_mul(3) / 5;
    fill_rect(
        framebuffer,
        width,
        0,
        horizon,
        width,
        height.saturating_sub(horizon),
        (18, 22, 34, 255),
    );

    let palette = [
        (28, 34, 56, 255),
        (36, 44, 68, 255),
        (42, 50, 82, 255),
        (24, 30, 46, 255),
    ];
    let mut x = 0usize;
    let mut idx = 0usize;
    while x < width {
        let bw = 28 + ((x / 17 + idx * 5) % 46);
        let bh = 60 + ((x / 23 + idx * 11) % 170);
        let top = horizon.saturating_sub(bh.min(horizon.saturating_sub(8)));
        let color = palette[idx % palette.len()];
        let actual_w = bw.min(width.saturating_sub(x));
        fill_rect(framebuffer, width, x, top, actual_w, horizon.saturating_sub(top), color);
        let win_color = blend_color(color, (250, 214, 118, 255), 88);
        let mut wy = top + 12;
        while wy + 4 < horizon {
            let mut wx = x + 6;
            while wx + 3 < x + actual_w {
                fill_rect(framebuffer, width, wx, wy, 3, 5, win_color);
                wx += 8;
            }
            wy += 12;
        }
        x += actual_w.saturating_sub(6).max(8);
        idx += 1;
    }

    let moon_x = width.saturating_sub(180);
    let moon_y = 64usize;
    let moon_w = 44.min(width.saturating_sub(moon_x));
    fill_rect(framebuffer, width, moon_x, moon_y, moon_w, 44, (242, 224, 170, 255));
    let crescent_x = moon_x + 10;
    let crescent_w = 44.min(width.saturating_sub(crescent_x));
    fill_rect(framebuffer, width, crescent_x, moon_y + 8, crescent_w, 44, (28, 22, 54, 255));
}

fn draw_box_cells(
    framebuffer: &mut [u8],
    width: usize,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    title: Option<&str>,
    border: Color,
    bg: Color,
) {
    if w == 0 || h == 0 {
        return;
    }
    let px = usize::from(x) * usize::from(CELL_WIDTH);
    let py = usize::from(y) * usize::from(CELL_HEIGHT);
    let pw = usize::from(w) * usize::from(CELL_WIDTH);
    let ph = usize::from(h) * usize::from(CELL_HEIGHT);
    let bg_rgba = color_to_rgba(bg, (0, 0, 0, 255));
    let border_rgba = color_to_rgba(border, (255, 255, 255, 255));
    let shadow = blend_color((0, 0, 0, 255), bg_rgba, 72);
    let title_bg = blend_color(bg_rgba, border_rgba, 26);
    let hi = blend_color(border_rgba, (255, 255, 255, 255), 84);
    let lo = blend_color(bg_rgba, (0, 0, 0, 255), 90);

    fill_rect(
        framebuffer,
        width,
        px + 4,
        py + 4,
        pw.saturating_sub(4),
        ph.saturating_sub(4),
        shadow,
    );
    fill_rect(framebuffer, width, px, py, pw, ph, bg_rgba);
    fill_rect(
        framebuffer,
        width,
        px,
        py,
        pw,
        usize::from(CELL_HEIGHT).min(ph),
        title_bg,
    );
    draw_rect_outline(framebuffer, width, px, py, pw, ph, border_rgba);
    if pw > 6 && ph > 6 {
        draw_rect_outline(
            framebuffer,
            width,
            px + 2,
            py + 2,
            pw.saturating_sub(4),
            ph.saturating_sub(4),
            blend_color(bg_rgba, hi, 48),
        );
    }
    draw_hline(framebuffer, width, px + 1, px + pw.saturating_sub(2), py + 1, hi);
    draw_vline(framebuffer, width, px + 1, py + 1, py + ph.saturating_sub(2), hi);
    draw_hline(
        framebuffer,
        width,
        px + 1,
        px + pw.saturating_sub(2),
        py + ph.saturating_sub(2),
        lo,
    );
    draw_vline(
        framebuffer,
        width,
        px + pw.saturating_sub(2),
        py + 1,
        py + ph.saturating_sub(2),
        lo,
    );
    if let Some(title) = title {
        draw_text_cells(
            framebuffer,
            width,
            x + 2,
            y,
            &fit_text(title, w.saturating_sub(4) as usize),
            border,
            Color::Reset,
        );
    }
}

fn draw_button_row(
    framebuffer: &mut [u8],
    width: usize,
    area: ClickArea,
    text: &str,
    selected: bool,
    selected_bg: Color,
    selected_fg: Color,
    normal_bg: Color,
    normal_fg: Color,
) {
    let bg = if selected { selected_bg } else { normal_bg };
    let fg = if selected { selected_fg } else { normal_fg };
    let bg_rgba = color_to_rgba(bg, (0, 0, 0, 255));
    let fg_rgba = color_to_rgba(fg, (255, 255, 255, 255));
    let px = usize::from(area.x) * usize::from(CELL_WIDTH);
    let py = usize::from(area.y) * usize::from(CELL_HEIGHT);
    let pw = usize::from(area.width) * usize::from(CELL_WIDTH);
    let ph = usize::from(area.height) * usize::from(CELL_HEIGHT);
    let border = if selected {
        blend_color(bg_rgba, fg_rgba, 96)
    } else {
        blend_color(bg_rgba, (255, 255, 255, 255), 32)
    };
    fill_rect(
        framebuffer,
        width,
        px + 2,
        py + 2,
        pw.saturating_sub(2),
        ph.saturating_sub(2),
        blend_color((0, 0, 0, 255), bg_rgba, 92),
    );
    fill_rect(framebuffer, width, px, py, pw, ph, bg_rgba);
    draw_rect_outline(framebuffer, width, px, py, pw, ph, border);
    if ph > 3 {
        fill_rect(
            framebuffer,
            width,
            px + 1,
            py + 1,
            pw.saturating_sub(2),
            (ph / 3).max(1),
            blend_color(bg_rgba, (255, 255, 255, 255), if selected { 38 } else { 18 }),
        );
    }
    let clipped = fit_text(text, area.width.saturating_sub(2) as usize);
    let start = area.x + area.width.saturating_sub(clipped.len() as u16) / 2;
    let row = area.y + area.height / 2;
    draw_text_cells(
        framebuffer,
        width,
        start,
        row,
        &clipped,
        fg,
        Color::Reset,
    );
}

fn draw_tool_button(
    framebuffer: &mut [u8],
    width: usize,
    area: ClickArea,
    label: &str,
    detail: &str,
    selected: bool,
    enabled: bool,
    accent: (u8, u8, u8, u8),
    ui: theme::UiPalette,
) {
    let bg = if selected {
        ui.toolbar_active_bg
    } else if enabled {
        ui.toolbar_button_bg
    } else {
        ui.window_bg
    };
    let fg = if selected {
        ui.toolbar_active_fg
    } else if enabled {
        ui.toolbar_button_fg
    } else {
        ui.text_dim
    };
    let bg_rgba = color_to_rgba(bg, (0, 0, 0, 255));
    let border = if selected {
        blend_color(bg_rgba, (255, 240, 196, 255), 112)
    } else {
        blend_color(bg_rgba, (255, 255, 255, 255), 40)
    };
    let px = usize::from(area.x) * usize::from(CELL_WIDTH);
    let py = usize::from(area.y) * usize::from(CELL_HEIGHT);
    let pw = usize::from(area.width) * usize::from(CELL_WIDTH);
    let ph = usize::from(area.height) * usize::from(CELL_HEIGHT);
    fill_rect(
        framebuffer,
        width,
        px + 2,
        py + 3,
        pw.saturating_sub(2),
        ph.saturating_sub(3),
        blend_color((0, 0, 0, 255), bg_rgba, 92),
    );
    fill_rect(framebuffer, width, px, py, pw, ph, bg_rgba);
    draw_rect_outline(framebuffer, width, px, py, pw, ph, border);
    fill_rect(
        framebuffer,
        width,
        px + 2,
        py + 2,
        (usize::from(CELL_WIDTH) / 2).max(4),
        ph.saturating_sub(4),
        if enabled {
            accent
        } else {
            blend_color(accent, bg_rgba, 148)
        },
    );
    fill_rect(
        framebuffer,
        width,
        px + 1,
        py + 1,
        pw.saturating_sub(2),
        (ph / 3).max(2),
        blend_color(bg_rgba, (255, 255, 255, 255), if selected { 40 } else { 20 }),
    );
    if selected {
        fill_rect(
            framebuffer,
            width,
            px + pw.saturating_sub(8),
            py + 2,
            5,
            5,
            accent,
        );
    }
    draw_text_cells(
        framebuffer,
        width,
        area.x + 2,
        area.y,
        &fit_text(label, area.width.saturating_sub(4) as usize),
        fg,
        Color::Reset,
    );
    if area.height > 1 {
        draw_text_cells(
            framebuffer,
            width,
            area.x + 2,
            area.y + 1,
            &fit_text(detail, area.width.saturating_sub(4) as usize),
            if !enabled {
                ui.text_dim
            } else if selected {
                ui.selection_fg
            } else {
                ui.text_dim
            },
            Color::Reset,
        );
    }
}

fn draw_centered_text_cells_scaled(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    row: u16,
    text: &str,
    scale: usize,
    fg: Color,
    bg: Color,
) {
    let clipped = fit_text(text, cols as usize / scale.max(1));
    let cell_w = clipped.len() as u16 * scale as u16;
    let start_col = cols.saturating_sub(cell_w) / 2;
    draw_text_cells_scaled(framebuffer, width, start_col, row, &clipped, scale, fg, bg);
}

fn draw_centered_text_cells(
    framebuffer: &mut [u8],
    width: usize,
    cols: u16,
    row: u16,
    text: &str,
    fg: Color,
    bg: Color,
) {
    let clipped = fit_text(text, cols as usize);
    let start = cols.saturating_sub(clipped.len() as u16) / 2;
    draw_text_cells(framebuffer, width, start, row, &clipped, fg, bg);
}

fn draw_text_cells(
    framebuffer: &mut [u8],
    width: usize,
    col: u16,
    row: u16,
    text: &str,
    fg: Color,
    bg: Color,
) {
    let bg_rgba = color_to_rgba(bg, (0, 0, 0, 255));
    let fg_rgba = color_to_rgba(fg, (255, 255, 255, 255));
    for (idx, ch) in text.chars().enumerate() {
        let x = col.saturating_add(idx as u16);
        if bg != Color::Reset {
            fill_cell_background(framebuffer, width, x, row, bg_rgba);
        }
        if ch != ' ' {
            draw_glyph(framebuffer, width, x, row, ch, fg_rgba);
        }
    }
}

fn draw_text_cells_scaled(
    framebuffer: &mut [u8],
    width: usize,
    col: u16,
    row: u16,
    text: &str,
    scale: usize,
    fg: Color,
    bg: Color,
) {
    let bg_rgba = color_to_rgba(bg, (0, 0, 0, 255));
    let fg_rgba = color_to_rgba(fg, (255, 255, 255, 255));
    for (idx, ch) in text.chars().enumerate() {
        let cell_x = usize::from(col) + idx * scale;
        let cell_y = usize::from(row);
        let px = cell_x * usize::from(CELL_WIDTH);
        let py = cell_y * usize::from(CELL_HEIGHT);
        let pw = usize::from(CELL_WIDTH) * scale;
        let ph = usize::from(CELL_HEIGHT) * scale;
        if bg != Color::Reset {
            fill_rect(framebuffer, width, px, py, pw, ph, bg_rgba);
        }
        if ch != ' ' {
            draw_scaled_glyph(framebuffer, width, px, py, ch, scale, fg_rgba);
        }
    }
}

fn fill_cell_rect(
    framebuffer: &mut [u8],
    width: usize,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    color: (u8, u8, u8, u8),
) {
    fill_rect(
        framebuffer,
        width,
        usize::from(x) * usize::from(CELL_WIDTH),
        usize::from(y) * usize::from(CELL_HEIGHT),
        usize::from(w) * usize::from(CELL_WIDTH),
        usize::from(h) * usize::from(CELL_HEIGHT),
        color,
    );
}

fn fit_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn format_menu_row_text(label: &str, right: &str, width: usize) -> String {
    if right.is_empty() || width <= label.len() + right.len() + 1 {
        return fit_text(label, width);
    }
    let padding = width.saturating_sub(label.len() + right.len());
    format!("{label}{}{right}", " ".repeat(padding))
}

fn wrap_text_lines(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let sep = if current.is_empty() { 0 } else { 1 };
        if current.chars().count() + sep + word_len > width {
            if !current.is_empty() {
                lines.push(current);
                if lines.len() == max_lines {
                    return lines;
                }
                current = String::new();
            }
            if word_len > width {
                lines.push(fit_text(word, width));
                if lines.len() == max_lines {
                    return lines;
                }
            } else {
                current.push_str(word);
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn layer_status_text(layer: ViewLayer) -> &'static str {
    match layer {
        ViewLayer::Surface => "Surface Layer",
        ViewLayer::Underground => "Underground",
    }
}

fn overlay_status_text(mode: OverlayMode) -> &'static str {
    match mode {
        OverlayMode::None => "Overlay Off",
        OverlayMode::Power => "Power Grid",
        OverlayMode::Water => "Water Service",
        OverlayMode::Traffic => "Traffic",
        OverlayMode::Pollution => "Pollution",
        OverlayMode::LandValue => "Land Value",
        OverlayMode::Crime => "Crime Rate",
        OverlayMode::FireRisk => "Fire Risk",
    }
}

fn chooser_kind_accent(kind: ToolChooserKind) -> (u8, u8, u8, u8) {
    match kind {
        ToolChooserKind::Zones => (86, 182, 104, 255),
        ToolChooserKind::Transport => (228, 168, 84, 255),
        ToolChooserKind::Utilities => (88, 170, 230, 255),
        ToolChooserKind::PowerPlants => (238, 210, 92, 255),
        ToolChooserKind::Buildings => (202, 128, 94, 255),
        ToolChooserKind::Terrain => (84, 180, 162, 255),
    }
}

fn toolbar_target_accent(target: ToolbarHitTarget) -> (u8, u8, u8, u8) {
    match target {
        ToolbarHitTarget::SelectTool(Tool::Inspect) => (146, 188, 244, 255),
        ToolbarHitTarget::SelectTool(Tool::Bulldoze) => (220, 126, 92, 255),
        ToolbarHitTarget::SelectTool(tool) => ToolChooserKind::for_tool(tool)
            .map(chooser_kind_accent)
            .unwrap_or((190, 190, 196, 255)),
        ToolbarHitTarget::OpenChooser(kind) => chooser_kind_accent(kind),
    }
}

fn capture_ingame_snapshot(app: &mut AppState) -> Option<PixelMapSnapshot> {
    let screen = app.screens.last_mut()?;
    let ingame = screen.as_any_mut().downcast_mut::<InGameScreen>()?;
    Some(PixelMapSnapshot {
        minimap: ingame.ui_areas.minimap,
        viewport: ingame.ui_areas.map.viewport,
        offset_x: ingame.camera.offset_x.max(0) as usize,
        offset_y: ingame.camera.offset_y.max(0) as usize,
        view_w: ingame.camera.view_w.max(1),
        view_h: ingame.camera.view_h.max(1),
        overlay_mode: ingame.overlay_mode,
        view_layer: ingame.view_layer,
    })
}

fn draw_square_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    map: &Map,
    snapshot: &PixelMapSnapshot,
    map_x: usize,
    map_y: usize,
    px: usize,
    py: usize,
    tile_size: usize,
) {
    let tile = rendered_tile(map, snapshot.view_layer, map_x, map_y);
    let overlay = map.get_overlay(map_x, map_y);
    let mut bg = tile_fill_color(tile, snapshot.view_layer);
    // Dry pipes get a neutral dark-grey background instead of blue.
    if tile == Tile::WaterPipe && overlay.water_service == 0 {
        bg = (35, 35, 40, 255);
    }
    if let Some(tint) = theme::overlay_tint(snapshot.overlay_mode, overlay) {
        bg = blend_color(bg, color_to_rgba(tint, bg), 96);
    }
    fill_rect(framebuffer, framebuffer_width, px, py, tile_size, tile_size, bg);
    let footprint = footprint_tile_role(map, snapshot.view_layer, map_x, map_y, tile);

    match tile {
        Tile::Water => draw_water_tile(framebuffer, framebuffer_width, px, py, tile_size),
        Tile::Trees => draw_trees_tile(framebuffer, framebuffer_width, px, py, tile_size),
        Tile::Dirt => draw_dirt_tile(framebuffer, framebuffer_width, px, py, tile_size),
        Tile::Road | Tile::RoadPowerLine | Tile::Onramp => {
            draw_network_tile(
                framebuffer,
                framebuffer_width,
                px,
                py,
                tile_size,
                (44, 46, 52, 255),
                (198, 194, 172, 255),
                connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                    neighbor.road_connects()
                }),
                2,
            );
            if tile == Tile::RoadPowerLine {
                draw_network_tile(
                    framebuffer,
                    framebuffer_width,
                    px,
                    py,
                    tile_size,
                    (0, 0, 0, 0),
                    (94, 212, 255, 255),
                    connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                        neighbor.power_connects()
                    }),
                    1,
                );
            } else if tile == Tile::Onramp {
                draw_onramp_tile(framebuffer, framebuffer_width, px, py, tile_size);
            }
        }
        Tile::Highway => draw_network_tile(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            (30, 32, 39, 255),
            (234, 210, 112, 255),
            connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                matches!(neighbor, Tile::Highway | Tile::Onramp)
            }),
            3,
        ),
        Tile::Rail => draw_rail_tile(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                neighbor.rail_connects()
            }),
        ),
        Tile::PowerLine => draw_network_tile(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            bg,
            (104, 222, 255, 255),
            connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                neighbor.power_connects()
            }),
            1,
        ),
        Tile::WaterPipe => {
            // Grey when dry, blue when connected to the water network.
            let pipe_color = if overlay.water_service > 0 {
                (83, 191, 255, 255)
            } else {
                (130, 130, 130, 255)
            };
            draw_network_tile(
                framebuffer,
                framebuffer_width,
                px,
                py,
                tile_size,
                bg,
                pipe_color,
                connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                    neighbor.water_connects()
                }),
                2,
            )
        }
        Tile::SubwayTunnel => draw_network_tile(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            bg,
            (218, 117, 240, 255),
            connection_mask(map, snapshot.view_layer, map_x, map_y, |neighbor| {
                neighbor.subway_connects() || neighbor == Tile::SubwayStation
            }),
                2,
        ),
        Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd => {
            let variant = building_variant_seed(map_x, map_y, tile);
            draw_zoned_lot_detail(framebuffer, framebuffer_width, px, py, tile_size, tile, variant);
        }
        Tile::ResLow
        | Tile::ResMed
        | Tile::ResHigh
        | Tile::CommLow
        | Tile::CommHigh
        | Tile::IndLight
        | Tile::IndHeavy
        | Tile::PowerPlantCoal
        | Tile::PowerPlantGas
        | Tile::PowerPlantNuclear
        | Tile::PowerPlantWind
        | Tile::PowerPlantSolar
        | Tile::Park
        | Tile::Police
        | Tile::Fire
        | Tile::Hospital
        | Tile::BusDepot
        | Tile::RailDepot
        | Tile::SubwayStation
        | Tile::WaterPump
        | Tile::WaterTower
        | Tile::WaterTreatment
        | Tile::Desalination
        | Tile::School
        | Tile::Stadium
        | Tile::Library => {
            let variant = building_variant_seed(map_x, map_y, tile);
            draw_building_detail(
                framebuffer,
                framebuffer_width,
                px,
                py,
                tile_size,
                tile,
                variant,
                footprint,
            );
        }
        Tile::Rubble => draw_rubble_tile(framebuffer, framebuffer_width, px, py, tile_size),
        _ => {
            if let Some(glyph) = tile_glyph(tile) {
                draw_centered_glyph(
                    framebuffer,
                    framebuffer_width,
                    px,
                    py,
                    tile_size,
                    tile_size,
                    glyph,
                    tile_glyph_color(tile),
                );
            }
        }
    }

    let border = blend_color(bg, (255, 255, 255, 255), 18);
    if let Some(role) = footprint {
        draw_tile_outer_edges(framebuffer, framebuffer_width, px, py, tile_size, role, border);
    } else {
        draw_rect_outline(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            tile_size,
            border,
        );
    }
}

fn rendered_tile(map: &Map, layer: ViewLayer, x: usize, y: usize) -> Tile {
    match layer {
        ViewLayer::Surface => map.surface_lot_tile(x, y),
        ViewLayer::Underground => map.view_tile(layer, x, y),
    }
}

fn building_variant_seed(map_x: usize, map_y: usize, tile: Tile) -> u8 {
    let tile_bias = tile as usize;
    (((map_x * 31) ^ (map_y * 17) ^ (tile_bias * 13)) & 0xFF) as u8
}

fn footprint_tile_role(
    map: &Map,
    layer: ViewLayer,
    map_x: usize,
    map_y: usize,
    tile: Tile,
) -> Option<FootprintTileRole> {
    let (w, h) = theme::tile_footprint_size(tile);
    if w <= 1 && h <= 1 {
        return None;
    }
    let start_x_min = map_x.saturating_sub(w.saturating_sub(1));
    let start_y_min = map_y.saturating_sub(h.saturating_sub(1));
    for start_y in start_y_min..=map_y {
        for start_x in start_x_min..=map_x {
            if start_x + w > map.width || start_y + h > map.height {
                continue;
            }
            let mut full_match = true;
            'tiles: for ty in 0..h {
                for tx in 0..w {
                    if rendered_tile(map, layer, start_x + tx, start_y + ty) != tile {
                        full_match = false;
                        break 'tiles;
                    }
                }
            }
            if full_match {
                return Some(FootprintTileRole {
                    dx: map_x - start_x,
                    dy: map_y - start_y,
                    w,
                    h,
                });
            }
        }
    }
    None
}

fn draw_zoned_lot_detail(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    tile: Tile,
    variant: u8,
) {
    let (stripe, accent) = match tile {
        Tile::ZoneRes => ((190, 244, 198, 255), (92, 156, 108, 255)),
        Tile::ZoneComm => ((190, 220, 255, 255), (88, 136, 196, 255)),
        Tile::ZoneInd => ((238, 216, 146, 255), (166, 132, 70, 255)),
        _ => ((255, 255, 255, 255), (200, 200, 200, 255)),
    };
    let inset = (tile_size / 6).max(1);
    fill_rect(
        framebuffer,
        framebuffer_width,
        px + inset,
        py + inset,
        tile_size.saturating_sub(inset * 2),
        tile_size.saturating_sub(inset * 2),
        blend_color(stripe, accent, 36),
    );
    match variant % 3 {
        0 => {
            for step in 0..3 {
                let y = py + inset + step * (tile_size / 4).max(2);
                let x0 = px + inset + step;
                let x1 = px + tile_size.saturating_sub(inset + 1 + step);
                if x0 < x1 && y < py + tile_size {
                    draw_hline(framebuffer, framebuffer_width, x0, x1, y, stripe);
                }
            }
        }
        1 => {
            let road = py + tile_size.saturating_sub((tile_size / 4).max(2));
            fill_rect(
                framebuffer,
                framebuffer_width,
                px + inset,
                road,
                tile_size.saturating_sub(inset * 2),
                (tile_size / 6).max(2),
                (68, 72, 76, 255),
            );
            let center_x = px + tile_size / 2;
            draw_vline(
                framebuffer,
                framebuffer_width,
                center_x,
                py + inset,
                road.saturating_sub(1),
                accent,
            );
        }
        _ => {
            for cx in [px + inset + 1, px + tile_size.saturating_sub(inset + 3)] {
                fill_rect(
                    framebuffer,
                    framebuffer_width,
                    cx,
                    py + inset + 1,
                    (tile_size / 6).max(2),
                    tile_size.saturating_sub(inset * 2 + 2),
                    stripe,
                );
            }
        }
    }
}

fn draw_building_detail(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    tile: Tile,
    variant: u8,
    footprint: Option<FootprintTileRole>,
) {
    let wall = blend_color(tile_fill_color(tile, ViewLayer::Surface), (245, 240, 230, 255), 18);
    let roof = blend_color(tile_fill_color(tile, ViewLayer::Surface), (28, 30, 34, 255), 48);
    let accent = match tile {
        Tile::Police => (94, 138, 255, 255),
        Tile::Fire => (255, 134, 110, 255),
        Tile::Hospital => (228, 72, 88, 255),
        Tile::PowerPlantWind | Tile::Park => (204, 245, 204, 255),
        Tile::WaterPump | Tile::WaterTower | Tile::WaterTreatment | Tile::Desalination => {
            (164, 224, 255, 255)
        }
        _ => (252, 236, 184, 255),
    };
    let inset = (tile_size / 5).max(2);
    let body_x = px + inset;
    let body_y = py + inset;
    let body_w = tile_size.saturating_sub(inset * 2);
    let body_h = tile_size.saturating_sub(inset * 2);
    if body_w < 4 || body_h < 4 {
        return;
    }
    if let Some(role) = footprint {
        draw_footprint_building_detail(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            tile,
            role,
            wall,
            roof,
            accent,
        );
        return;
    }
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, wall);
    match tile {
        Tile::ResLow | Tile::ResMed | Tile::ResHigh => {
            draw_residential_building(
                framebuffer,
                framebuffer_width,
                px,
                py,
                tile_size,
                wall,
                roof,
                accent,
                variant,
            );
        }
        Tile::CommLow | Tile::CommHigh => {
            draw_commercial_building(
                framebuffer,
                framebuffer_width,
                px,
                py,
                tile_size,
                wall,
                roof,
                accent,
                variant,
            );
        }
        Tile::IndLight | Tile::IndHeavy => {
            draw_industrial_building(
                framebuffer,
                framebuffer_width,
                px,
                py,
                tile_size,
                wall,
                roof,
                accent,
                variant,
            );
        }
        _ => {
            let roof_h = (body_h / 4).max(2);
            fill_rect(
                framebuffer,
                framebuffer_width,
                body_x,
                body_y,
                body_w,
                roof_h,
                roof,
            );
            let win = (tile_size / 8).max(1);
            let gap = (tile_size / 6).max(2);
            let start_y = body_y + (body_h / 3).max(2);
            let max_y = body_y + body_h.saturating_sub(win + 1);
            let max_x = body_x + body_w.saturating_sub(win + 1);
            let mut wy = start_y;
            while wy <= max_y {
                let mut wx = body_x + gap / 2;
                while wx <= max_x {
                    fill_rect(framebuffer, framebuffer_width, wx, wy, win, win, accent);
                    wx += gap;
                }
                wy += gap;
            }
            if tile_size >= 12 {
                let door_w = (body_w / 4).max(2);
                let door_x = body_x + body_w.saturating_sub(door_w) / 2;
                fill_rect(
                    framebuffer,
                    framebuffer_width,
                    door_x,
                    body_y + body_h.saturating_sub((body_h / 4).max(2)),
                    door_w,
                    (body_h / 4).max(2),
                    roof,
                );
            }
        }
    }
}

fn draw_onramp_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
) {
    let lane = (252, 189, 73, 255);
    let mid_y = py + tile_size / 2;
    let start_x = px + (tile_size / 4).max(2);
    let end_x = px + tile_size.saturating_sub((tile_size / 4).max(2));
    draw_hline(framebuffer, framebuffer_width, start_x, end_x, mid_y, lane);
    let head = (tile_size / 4).max(2);
    for step in 0..head {
        let x = end_x.saturating_sub(step);
        let up = mid_y.saturating_sub(step);
        let down = (mid_y + step).min(py + tile_size.saturating_sub(1));
        set_pixel(framebuffer, framebuffer_width, x, up, lane);
        set_pixel(framebuffer, framebuffer_width, x, down, lane);
    }
}

fn draw_tile_outer_edges(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    role: FootprintTileRole,
    color: (u8, u8, u8, u8),
) {
    let max_x = px + tile_size.saturating_sub(1);
    let max_y = py + tile_size.saturating_sub(1);
    if role.dy == 0 {
        draw_hline(framebuffer, framebuffer_width, px, max_x, py, color);
    }
    if role.dy + 1 == role.h {
        draw_hline(framebuffer, framebuffer_width, px, max_x, max_y, color);
    }
    if role.dx == 0 {
        draw_vline(framebuffer, framebuffer_width, px, py, max_y, color);
    }
    if role.dx + 1 == role.w {
        draw_vline(framebuffer, framebuffer_width, max_x, py, max_y, color);
    }
}

fn draw_residential_building(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    wall: (u8, u8, u8, u8),
    roof: (u8, u8, u8, u8),
    accent: (u8, u8, u8, u8),
    variant: u8,
) {
    let inset = (tile_size / 6).max(1);
    let body_x = px + inset;
    let body_y = py + inset + (tile_size / 10);
    let body_w = tile_size.saturating_sub(inset * 2);
    let body_h = tile_size.saturating_sub(inset * 2 + (tile_size / 10));
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, wall);
    let roof_h = (body_h / 4).max(2);
    match variant % 3 {
        0 => {
            fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, roof_h, roof);
        }
        1 => {
            for step in 0..roof_h {
                let x0 = body_x + step.min(body_w / 2);
                let x1 = body_x + body_w.saturating_sub(step + 1);
                let y = body_y + step;
                if x0 < x1 {
                    draw_hline(framebuffer, framebuffer_width, x0, x1, y, roof);
                }
            }
        }
        _ => {
            let left_w = body_w / 2;
            fill_rect(framebuffer, framebuffer_width, body_x, body_y, left_w, roof_h, roof);
            fill_rect(
                framebuffer,
                framebuffer_width,
                body_x + left_w.saturating_sub(1),
                body_y + roof_h / 2,
                body_w.saturating_sub(left_w.saturating_sub(1)),
                roof_h,
                roof,
            );
        }
    }
    let win = (tile_size / 7).max(1);
    let top = body_y + roof_h + 1;
    for row in 0..2 {
        let wy = top + row * ((tile_size / 5).max(2));
        for col in 0..2 {
            let wx = body_x + (tile_size / 7).max(2) + col * ((tile_size / 4).max(3));
            fill_rect(framebuffer, framebuffer_width, wx, wy, win, win, accent);
        }
    }
    let door_w = (body_w / 4).max(2);
    let door_h = (body_h / 4).max(2);
    let door_x = body_x + body_w / 2 - door_w / 2;
    let door_y = body_y + body_h.saturating_sub(door_h);
    fill_rect(framebuffer, framebuffer_width, door_x, door_y, door_w, door_h, roof);
}

fn draw_commercial_building(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    wall: (u8, u8, u8, u8),
    roof: (u8, u8, u8, u8),
    accent: (u8, u8, u8, u8),
    variant: u8,
) {
    let inset = (tile_size / 6).max(1);
    let body_x = px + inset;
    let body_y = py + inset;
    let body_w = tile_size.saturating_sub(inset * 2);
    let body_h = tile_size.saturating_sub(inset * 2);
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, wall);
    let roof_h = (body_h / 5).max(2);
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, roof_h, roof);
    let sign_h = (tile_size / 8).max(2);
    let sign_y = body_y + roof_h + 1;
    let sign = match variant % 3 {
        0 => accent,
        1 => blend_color(accent, (255, 198, 92, 255), 96),
        _ => blend_color(accent, (255, 255, 255, 255), 72),
    };
    fill_rect(
        framebuffer,
        framebuffer_width,
        body_x + 1,
        sign_y,
        body_w.saturating_sub(2),
        sign_h,
        sign,
    );
    let glass = blend_color(accent, (255, 255, 255, 255), 72);
    let cols = if body_w >= 10 { 3 } else { 2 };
    for col in 0..cols {
        let wx = body_x + 1 + col * ((body_w.saturating_sub(2)) / cols.max(1));
        fill_rect(
            framebuffer,
            framebuffer_width,
            wx,
            sign_y + sign_h + 1,
            (body_w / 5).max(2),
            body_h.saturating_sub(sign_h + roof_h + (tile_size / 5).max(3)),
            glass,
        );
    }
    let awning_y = body_y + body_h.saturating_sub((tile_size / 5).max(2));
    fill_rect(
        framebuffer,
        framebuffer_width,
        body_x,
        awning_y,
        body_w,
        (tile_size / 10).max(2),
        roof,
    );
}

fn draw_industrial_building(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    wall: (u8, u8, u8, u8),
    roof: (u8, u8, u8, u8),
    accent: (u8, u8, u8, u8),
    variant: u8,
) {
    let inset = (tile_size / 7).max(1);
    let body_x = px + inset;
    let body_y = py + inset + (tile_size / 10);
    let body_w = tile_size.saturating_sub(inset * 2);
    let body_h = tile_size.saturating_sub(inset * 2 + (tile_size / 10));
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, wall);
    let roof_h = (body_h / 4).max(2);
    for step in 0..3 {
        let seg_w = (body_w / 3).max(2);
        let seg_x = body_x + step * seg_w;
        let offset = if (step + variant as usize) % 2 == 0 { 0 } else { roof_h / 2 };
        fill_rect(
            framebuffer,
            framebuffer_width,
            seg_x,
            body_y + offset,
            seg_w,
            roof_h,
            roof,
        );
    }
    let vent_w = (tile_size / 8).max(2);
    let vent_x = body_x + body_w.saturating_sub(vent_w + 2);
    fill_rect(
        framebuffer,
        framebuffer_width,
        vent_x,
        py + inset,
        vent_w,
        body_h / 2,
        blend_color(roof, (255, 255, 255, 255), 32),
    );
    let smoke = blend_color(accent, (220, 220, 220, 255), 90);
    for puff in 0..3 {
        let puff_x = vent_x.saturating_sub(puff);
        let puff_y = py + inset.saturating_sub(1) + puff * 2;
        fill_rect(
            framebuffer,
            framebuffer_width,
            puff_x,
            puff_y,
            (tile_size / 10).max(1),
            (tile_size / 10).max(1),
            smoke,
        );
    }
    let bay_y = body_y + body_h.saturating_sub((tile_size / 5).max(2));
    fill_rect(
        framebuffer,
        framebuffer_width,
        body_x + 2,
        bay_y,
        body_w.saturating_sub(4),
        (tile_size / 6).max(2),
        accent,
    );
}

fn draw_footprint_building_detail(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    tile: Tile,
    role: FootprintTileRole,
    wall: (u8, u8, u8, u8),
    roof: (u8, u8, u8, u8),
    accent: (u8, u8, u8, u8),
) {
    let inset = (tile_size / 8).max(1);
    let body_x = px + inset;
    let body_y = py + inset;
    let body_w = tile_size.saturating_sub(inset * 2);
    let body_h = tile_size.saturating_sub(inset * 2);
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, wall);

    let roof_h = (body_h / 5).max(2);
    if role.dy == 0 {
        fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, roof_h, roof);
    } else {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            (roof_h / 2).max(1),
            blend_color(roof, wall, 80),
        );
    }

    if role.dx == 0 {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            (tile_size / 10).max(1),
            body_h,
            blend_color(wall, roof, 26),
        );
    }
    if role.dx + 1 == role.w {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + body_w.saturating_sub((tile_size / 10).max(1)),
            body_y,
            (tile_size / 10).max(1),
            body_h,
            blend_color(roof, wall, 34),
        );
    }

    match tile {
        Tile::Stadium => draw_stadium_tile(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            body_h,
            role,
        ),
        Tile::PowerPlantCoal | Tile::PowerPlantGas | Tile::PowerPlantNuclear => {
            draw_power_plant_tile(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            body_x,
            body_y,
            body_w,
            body_h,
            role,
            tile,
            roof,
            accent,
        )
        }
        Tile::Police | Tile::Fire => draw_civic_compound_tile(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            body_h,
            role,
            tile,
            accent,
        ),
        Tile::WaterTreatment | Tile::Desalination => draw_waterworks_tile(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            body_h,
            role,
            tile,
            accent,
        ),
        Tile::WaterTower => draw_water_tower_tile(
            framebuffer,
            framebuffer_width,
            px,
            py,
            tile_size,
            role,
            accent,
        ),
        Tile::PowerPlantSolar => draw_solar_array_tile(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            body_h,
            accent,
        ),
        Tile::Park => draw_park_tile(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            body_h,
            role,
        ),
        Tile::BusDepot | Tile::RailDepot => draw_depot_tile(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y,
            body_w,
            body_h,
            role,
            tile,
            accent,
        ),
        _ => {
            let win = (tile_size / 9).max(1);
            for row in 0..2 {
                let wy = body_y + roof_h + 2 + row * ((tile_size / 5).max(2));
                for col in 0..2 {
                    let wx = body_x + 2 + col * ((tile_size / 4).max(2));
                    fill_rect(framebuffer, framebuffer_width, wx, wy, win, win, accent);
                }
            }
        }
    }
}

fn draw_stadium_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    role: FootprintTileRole,
) {
    let turf = (88, 170, 94, 255);
    let seats = (220, 228, 236, 255);
    if role.dx > 0 && role.dx + 1 < role.w && role.dy > 0 && role.dy + 1 < role.h {
        fill_rect(framebuffer, framebuffer_width, body_x + 1, body_y + 1, body_w.saturating_sub(2), body_h.saturating_sub(2), turf);
        if body_w > 6 && role.dy == role.h / 2 {
            let mid_y = body_y + body_h / 2;
            draw_hline(framebuffer, framebuffer_width, body_x + 2, body_x + body_w.saturating_sub(3), mid_y, (235, 245, 235, 255));
        }
    } else {
        let band = (body_h / 5).max(2);
        if role.dy == 0 || role.dy + 1 == role.h {
            fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, band, seats);
        }
        if role.dx == 0 || role.dx + 1 == role.w {
            fill_rect(framebuffer, framebuffer_width, body_x, body_y, band, body_h, seats);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_power_plant_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    role: FootprintTileRole,
    tile: Tile,
    roof: (u8, u8, u8, u8),
    accent: (u8, u8, u8, u8),
) {
    let pipe = blend_color(accent, (210, 210, 210, 255), 80);
    if role.dy + 1 == role.h {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y + body_h.saturating_sub((tile_size / 6).max(2)),
            body_w,
            (tile_size / 6).max(2),
            roof,
        );
    }
    if role.dx + 1 == role.w && role.dy <= 1 {
        let stack_w = (tile_size / 6).max(2);
        let stack_h = (tile_size / 2).max(4);
        let stack_x = px + tile_size.saturating_sub(stack_w + 3);
        let stack_y = py + (tile_size / 6).max(1);
        fill_rect(framebuffer, framebuffer_width, stack_x, stack_y, stack_w, stack_h, pipe);
        let smoke = match tile {
            Tile::PowerPlantCoal => (120, 120, 120, 255),
            Tile::PowerPlantGas => (192, 196, 204, 255),
            Tile::PowerPlantNuclear => (226, 236, 244, 255),
            _ => (180, 180, 180, 255),
        };
        for puff in 0..3 {
            fill_rect(
                framebuffer,
                framebuffer_width,
                stack_x.saturating_sub(puff),
                stack_y.saturating_sub((puff + 1) * 2),
                (tile_size / 8).max(1),
                (tile_size / 8).max(1),
                smoke,
            );
        }
    } else if role.dy == 1 {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + 1,
            body_y + body_h / 2,
            body_w.saturating_sub(2),
            (tile_size / 10).max(1),
            pipe,
        );
    }
}

fn draw_civic_compound_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    role: FootprintTileRole,
    tile: Tile,
    accent: (u8, u8, u8, u8),
) {
    let stripe = if tile == Tile::Police {
        (96, 146, 220, 255)
    } else {
        (224, 94, 84, 255)
    };
    if role.dy == 0 {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x,
            body_y + 1,
            body_w,
            (body_h / 6).max(2),
            stripe,
        );
    }
    if role.dx == role.w / 2 && role.dy == role.h / 2 {
        let badge = (body_w / 3).max(2);
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + body_w / 2 - badge / 2,
            body_y + body_h / 2 - badge / 2,
            badge,
            badge,
            accent,
        );
    }
    if role.dy + 1 == role.h && role.dx == role.w / 2 {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + body_w / 3,
            body_y + body_h.saturating_sub((body_h / 4).max(2)),
            body_w / 3,
            (body_h / 4).max(2),
            stripe,
        );
    }
}

fn draw_waterworks_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    role: FootprintTileRole,
    tile: Tile,
    accent: (u8, u8, u8, u8),
) {
    let basin = if tile == Tile::Desalination {
        (110, 184, 220, 255)
    } else {
        (88, 152, 196, 255)
    };
    if role.dx + 1 < role.w {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + 1,
            body_y + body_h / 3,
            body_w.saturating_sub(2),
            body_h / 3,
            basin,
        );
    }
    if role.dx == role.w.saturating_sub(1) {
        let tank = (body_w / 3).max(2);
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + body_w / 2 - tank / 2,
            body_y + body_h / 4,
            tank,
            body_h / 2,
            accent,
        );
    }
}

fn draw_water_tower_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    role: FootprintTileRole,
    accent: (u8, u8, u8, u8),
) {
    let tank = (184, 212, 230, 255);
    if role.dy == 0 {
        fill_rect(
            framebuffer,
            framebuffer_width,
            px + tile_size / 5,
            py + tile_size / 5,
            tile_size.saturating_sub((tile_size / 5) * 2),
            (tile_size / 4).max(3),
            tank,
        );
    }
    if role.dy + 1 == role.h {
        let leg_x = if role.dx == 0 {
            px + tile_size / 3
        } else {
            px + tile_size / 2
        };
        fill_rect(
            framebuffer,
            framebuffer_width,
            leg_x,
            py + tile_size / 4,
            (tile_size / 10).max(2),
            tile_size / 2,
            accent,
        );
    }
}

fn draw_solar_array_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    accent: (u8, u8, u8, u8),
) {
    let panel = (56, 100, 188, 255);
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, panel);
    let rows = 3;
    for row in 0..rows {
        let y = body_y + row * (body_h / rows.max(1));
        draw_hline(framebuffer, framebuffer_width, body_x, body_x + body_w.saturating_sub(1), y, accent);
    }
    let cols = 3;
    for col in 0..cols {
        let x = body_x + col * (body_w / cols.max(1));
        draw_vline(framebuffer, framebuffer_width, x, body_y, body_y + body_h.saturating_sub(1), accent);
    }
}

fn draw_park_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    role: FootprintTileRole,
) {
    let grass = (92, 176, 96, 255);
    let tree = (38, 104, 44, 255);
    fill_rect(framebuffer, framebuffer_width, body_x, body_y, body_w, body_h, grass);
    if role.dx == role.dy || role.dx + role.dy + 1 == role.w.max(role.h) {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + body_w / 3,
            body_y + body_h / 3,
            (body_w / 3).max(2),
            (body_h / 3).max(2),
            tree,
        );
    } else {
        draw_hline(
            framebuffer,
            framebuffer_width,
            body_x + 1,
            body_x + body_w.saturating_sub(2),
            body_y + body_h / 2,
            (224, 214, 166, 255),
        );
    }
}

fn draw_depot_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    body_x: usize,
    body_y: usize,
    body_w: usize,
    body_h: usize,
    role: FootprintTileRole,
    tile: Tile,
    accent: (u8, u8, u8, u8),
) {
    let guide = if tile == Tile::RailDepot {
        (184, 184, 184, 255)
    } else {
        (232, 196, 104, 255)
    };
    let lines = if tile == Tile::RailDepot { 2 } else { 3 };
    for line in 0..lines {
        let y = body_y + 2 + line * ((body_h.saturating_sub(4)) / lines.max(1));
        draw_hline(framebuffer, framebuffer_width, body_x + 1, body_x + body_w.saturating_sub(2), y, guide);
    }
    if role.dx == role.w / 2 {
        fill_rect(
            framebuffer,
            framebuffer_width,
            body_x + body_w / 3,
            body_y + body_h.saturating_sub((body_h / 4).max(2)),
            body_w / 3,
            (body_h / 4).max(2),
            accent,
        );
    }
}

fn square_minimap_layout(area: ClickArea, map_width: usize, map_height: usize) -> Option<SquareGridLayout> {
    if area.width == 0 || area.height == 0 || map_width == 0 || map_height == 0 {
        return None;
    }

    let area_x = usize::from(area.x) * usize::from(CELL_WIDTH);
    let area_y = usize::from(area.y) * usize::from(CELL_HEIGHT);
    let area_w = usize::from(area.width) * usize::from(CELL_WIDTH);
    let area_h = usize::from(area.height) * usize::from(CELL_HEIGHT);
    if area_w == 0 || area_h == 0 {
        return None;
    }

    let tile_size = (area_w / map_width.max(1)).min(area_h / map_height.max(1)).max(1);
    let grid_w = map_width.min((area_w / tile_size).max(1));
    let grid_h = map_height.min((area_h / tile_size).max(1));
    let width = grid_w * tile_size;
    let height = grid_h * tile_size;
    Some(SquareGridLayout {
        x: area_x + area_w.saturating_sub(width) / 2,
        y: area_y + area_h.saturating_sub(height) / 2,
        tile_size,
        grid_w,
        grid_h,
    })
}

fn square_map_layout(area: ClickArea, view_w: usize, view_h: usize) -> Option<SquareGridLayout> {
    if area.width == 0 || area.height == 0 || view_w == 0 || view_h == 0 {
        return None;
    }

    let area_x = usize::from(area.x) * usize::from(CELL_WIDTH);
    let area_y = usize::from(area.y) * usize::from(CELL_HEIGHT);
    let area_w = usize::from(area.width) * usize::from(CELL_WIDTH);
    let area_h = usize::from(area.height) * usize::from(CELL_HEIGHT);
    if area_w == 0 || area_h == 0 {
        return None;
    }

    let tile_size = (area_w / view_w.max(1)).min(area_h / view_h.max(1)).max(1);
    let width = view_w * tile_size;
    let height = view_h * tile_size;
    Some(SquareGridLayout {
        x: area_x + area_w.saturating_sub(width) / 2,
        y: area_y + area_h.saturating_sub(height) / 2,
        tile_size,
        grid_w: view_w,
        grid_h: view_h,
    })
}

fn square_layout_tile_at_pixel(
    layout: SquareGridLayout,
    map_width: usize,
    map_height: usize,
    px: usize,
    py: usize,
) -> Option<(usize, usize)> {
    if px < layout.x
        || px >= layout.x + layout.grid_w * layout.tile_size
        || py < layout.y
        || py >= layout.y + layout.grid_h * layout.tile_size
    {
        return None;
    }

    let rel_x = (px - layout.x) / layout.tile_size;
    let rel_y = (py - layout.y) / layout.tile_size;
    let tile_x = if layout.grid_w <= 1 {
        0
    } else {
        rel_x * map_width.saturating_sub(1) / (layout.grid_w - 1)
    };
    let tile_y = if layout.grid_h <= 1 {
        0
    } else {
        rel_y * map_height.saturating_sub(1) / (layout.grid_h - 1)
    };
    Some((tile_x, tile_y))
}

fn synthetic_minimap_col(area: ClickArea, map_width: usize, tile_x: usize) -> u16 {
    let grid_w = (area.width / 2).max(1) as usize;
    let rel_x = if map_width <= 1 || grid_w <= 1 {
        0
    } else {
        tile_x.min(map_width.saturating_sub(1)) * (grid_w - 1) / (map_width - 1)
    };
    area.x + (rel_x as u16).saturating_mul(2)
}

fn synthetic_minimap_row(area: ClickArea, map_height: usize, tile_y: usize) -> u16 {
    let grid_h = area.height.max(1) as usize;
    let rel_y = if map_height <= 1 || grid_h <= 1 {
        0
    } else {
        tile_y.min(map_height.saturating_sub(1)) * (grid_h - 1) / (map_height - 1)
    };
    area.y + rel_y as u16
}

fn connection_mask<F>(map: &Map, layer: ViewLayer, x: usize, y: usize, matches_neighbor: F) -> ConnectionMask
where
    F: Fn(Tile) -> bool,
{
    let left = x > 0 && matches_neighbor(map.view_tile(layer, x - 1, y));
    let right = x + 1 < map.width && matches_neighbor(map.view_tile(layer, x + 1, y));
    let up = y > 0 && matches_neighbor(map.view_tile(layer, x, y - 1));
    let down = y + 1 < map.height && matches_neighbor(map.view_tile(layer, x, y + 1));
    ConnectionMask {
        left,
        right,
        up,
        down,
    }
}

fn tile_fill_color(tile: Tile, layer: ViewLayer) -> (u8, u8, u8, u8) {
    if layer == ViewLayer::Underground {
        return match tile {
            Tile::WaterPipe => (18, 54, 88, 255),
            Tile::SubwayTunnel => (52, 34, 66, 255),
            Tile::SubwayStation => (69, 60, 88, 255),
            _ => (44, 31, 24, 255),
        };
    }

    match tile {
        Tile::Grass => (68, 124, 62, 255),
        Tile::Trees => (44, 96, 42, 255),
        Tile::Water => (24, 88, 148, 255),
        Tile::Dirt => (112, 88, 52, 255),
        Tile::Road | Tile::RoadPowerLine | Tile::Onramp => (38, 40, 46, 255),
        Tile::Rail => (76, 60, 42, 255),
        Tile::PowerLine => (58, 80, 66, 255),
        Tile::Highway => (28, 30, 36, 255),
        Tile::WaterPipe => (18, 54, 88, 255),
        Tile::SubwayTunnel => (52, 34, 66, 255),
        Tile::ZoneRes => (58, 120, 78, 255),
        Tile::ZoneComm => (58, 92, 140, 255),
        Tile::ZoneInd => (124, 112, 58, 255),
        Tile::ResLow => (86, 156, 92, 255),
        Tile::ResMed => (64, 132, 74, 255),
        Tile::ResHigh => (38, 92, 58, 255),
        Tile::CommLow => (78, 112, 176, 255),
        Tile::CommHigh => (52, 84, 150, 255),
        Tile::IndLight => (148, 128, 70, 255),
        Tile::IndHeavy => (126, 94, 48, 255),
        Tile::PowerPlantCoal | Tile::PowerPlantGas | Tile::PowerPlantNuclear => {
            (86, 86, 92, 255)
        }
        Tile::PowerPlantWind => (76, 100, 122, 255),
        Tile::PowerPlantSolar => (100, 92, 54, 255),
        Tile::Park => (44, 132, 66, 255),
        Tile::Police => (56, 84, 166, 255),
        Tile::Fire => (170, 68, 54, 255),
        Tile::Hospital => (168, 168, 176, 255),
        Tile::BusDepot => (138, 112, 54, 255),
        Tile::RailDepot => (118, 88, 60, 255),
        Tile::SubwayStation => (86, 68, 118, 255),
        Tile::WaterPump | Tile::WaterTower | Tile::WaterTreatment | Tile::Desalination => {
            (62, 114, 150, 255)
        }
        Tile::School => (188, 136, 76, 255),
        Tile::Stadium => (84, 110, 96, 255),
        Tile::Library => (104, 78, 58, 255),
        Tile::Rubble => (96, 82, 74, 255),
    }
}

fn tile_glyph(tile: Tile) -> Option<char> {
    match tile {
        Tile::Grass | Tile::Water | Tile::Trees | Tile::Dirt => None,
        Tile::ZoneRes => Some('R'),
        Tile::ZoneComm => Some('C'),
        Tile::ZoneInd => Some('I'),
        Tile::ResLow => Some('h'),
        Tile::ResMed => Some('H'),
        Tile::ResHigh => Some('A'),
        Tile::CommLow => Some('c'),
        Tile::CommHigh => Some('C'),
        Tile::IndLight => Some('i'),
        Tile::IndHeavy => Some('I'),
        Tile::PowerPlantCoal => Some('C'),
        Tile::PowerPlantGas => Some('G'),
        Tile::PowerPlantNuclear => Some('N'),
        Tile::PowerPlantWind => Some('W'),
        Tile::PowerPlantSolar => Some('S'),
        Tile::Park => Some('*'),
        Tile::Police => Some('P'),
        Tile::Fire => Some('F'),
        Tile::Hospital => Some('H'),
        Tile::BusDepot => Some('B'),
        Tile::RailDepot => Some('R'),
        Tile::SubwayStation => Some('M'),
        Tile::WaterPump => Some('P'),
        Tile::WaterTower => Some('T'),
        Tile::WaterTreatment => Some('W'),
        Tile::Desalination => Some('D'),
        Tile::School => Some('S'),
        Tile::Stadium => Some('U'),
        Tile::Library => Some('L'),
        Tile::Rubble => Some('x'),
        _ => None,
    }
}

fn tile_glyph_color(tile: Tile) -> (u8, u8, u8, u8) {
    match tile {
        Tile::Hospital => (180, 24, 36, 255),
        Tile::Park | Tile::PowerPlantWind => (220, 244, 220, 255),
        Tile::WaterPump | Tile::WaterTower | Tile::WaterTreatment | Tile::Desalination => {
            (210, 240, 255, 255)
        }
        _ => (245, 240, 230, 255),
    }
}

fn draw_water_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
) {
    let wave = (122, 198, 255, 255);
    let y0 = py + tile_size / 4;
    let y1 = py + (tile_size * 3) / 4;
    draw_hline(framebuffer, framebuffer_width, px + 2, px + tile_size.saturating_sub(3), y0, wave);
    draw_hline(framebuffer, framebuffer_width, px + 1, px + tile_size.saturating_sub(2), y1, wave);
}

fn draw_trees_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
) {
    let leaf = (28, 64, 28, 255);
    let trunk = (96, 66, 34, 255);
    let center_x = px + tile_size / 2;
    let center_y = py + tile_size / 2;
    fill_rect(framebuffer, framebuffer_width, center_x.saturating_sub(3), center_y.saturating_sub(4), 6, 6, leaf);
    fill_rect(framebuffer, framebuffer_width, center_x.saturating_sub(1), center_y + 1, 2, 4, trunk);
}

fn draw_dirt_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
) {
    let ridge = (142, 116, 68, 255);
    let pebble = (98, 76, 44, 255);
    let step = (tile_size / 4).max(3);
    let mut y = py + 2;
    while y < py + tile_size.saturating_sub(2) {
        draw_hline(
            framebuffer,
            framebuffer_width,
            px + 1 + ((y - py) / step) % 3,
            px + tile_size.saturating_sub(3),
            y,
            ridge,
        );
        y += step;
    }
    fill_rect(
        framebuffer,
        framebuffer_width,
        px + tile_size / 4,
        py + tile_size / 3,
        (tile_size / 8).max(1),
        (tile_size / 8).max(1),
        pebble,
    );
    fill_rect(
        framebuffer,
        framebuffer_width,
        px + (tile_size * 2) / 3,
        py + (tile_size * 3) / 5,
        (tile_size / 7).max(1),
        (tile_size / 7).max(1),
        pebble,
    );
}

fn draw_rubble_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
) {
    let slab = (132, 118, 108, 255);
    let dust = (82, 70, 62, 255);
    let chunks = [
        (tile_size / 6, tile_size / 2, tile_size / 4, tile_size / 5),
        (tile_size / 2, tile_size / 3, tile_size / 5, tile_size / 6),
        ((tile_size * 3) / 5, (tile_size * 2) / 3, tile_size / 4, tile_size / 6),
    ];
    for (dx, dy, w, h) in chunks {
        fill_rect(
            framebuffer,
            framebuffer_width,
            px + dx,
            py + dy,
            w.max(2),
            h.max(2),
            slab,
        );
    }
    draw_hline(
        framebuffer,
        framebuffer_width,
        px + 2,
        px + tile_size.saturating_sub(3),
        py + tile_size.saturating_sub(tile_size / 4).max(2),
        dust,
    );
}

fn draw_network_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    fill: (u8, u8, u8, u8),
    line: (u8, u8, u8, u8),
    mask: ConnectionMask,
    thickness: usize,
) {
    if fill.3 > 0 {
        fill_rect(framebuffer, framebuffer_width, px, py, tile_size, tile_size, fill);
    }
    let cx = px + tile_size / 2;
    let cy = py + tile_size / 2;
    let half = thickness / 2;
    fill_rect(
        framebuffer,
        framebuffer_width,
        cx.saturating_sub(half),
        cy.saturating_sub(half),
        thickness.max(1),
        thickness.max(1),
        line,
    );
    if mask.left {
        fill_rect(
            framebuffer,
            framebuffer_width,
            px,
            cy.saturating_sub(half),
            tile_size / 2 + 1,
            thickness.max(1),
            line,
        );
    }
    if mask.right {
        fill_rect(
            framebuffer,
            framebuffer_width,
            cx,
            cy.saturating_sub(half),
            tile_size / 2,
            thickness.max(1),
            line,
        );
    }
    if mask.up {
        fill_rect(
            framebuffer,
            framebuffer_width,
            cx.saturating_sub(half),
            py,
            thickness.max(1),
            tile_size / 2 + 1,
            line,
        );
    }
    if mask.down {
        fill_rect(
            framebuffer,
            framebuffer_width,
            cx.saturating_sub(half),
            cy,
            thickness.max(1),
            tile_size / 2,
            line,
        );
    }
}

fn draw_rail_tile(
    framebuffer: &mut [u8],
    framebuffer_width: usize,
    px: usize,
    py: usize,
    tile_size: usize,
    mask: ConnectionMask,
) {
    let rail = (212, 214, 220, 255);
    let ties = (116, 76, 42, 255);
    let inset = tile_size.max(6) / 4;
    let cx = px + tile_size / 2;
    let cy = py + tile_size / 2;
    if mask.left || mask.right || (!mask.up && !mask.down) {
        draw_hline(framebuffer, framebuffer_width, px + 1, px + tile_size.saturating_sub(2), cy.saturating_sub(2), rail);
        draw_hline(framebuffer, framebuffer_width, px + 1, px + tile_size.saturating_sub(2), cy + 2, rail);
        for offset in (px + 1..px + tile_size.saturating_sub(1)).step_by(inset.max(3)) {
            draw_vline(framebuffer, framebuffer_width, offset, cy.saturating_sub(4), cy + 4, ties);
        }
    }
    if mask.up || mask.down || (!mask.left && !mask.right) {
        draw_vline(framebuffer, framebuffer_width, cx.saturating_sub(2), py + 1, py + tile_size.saturating_sub(2), rail);
        draw_vline(framebuffer, framebuffer_width, cx + 2, py + 1, py + tile_size.saturating_sub(2), rail);
        for offset in (py + 1..py + tile_size.saturating_sub(1)).step_by(inset.max(3)) {
            draw_hline(framebuffer, framebuffer_width, cx.saturating_sub(4), cx + 4, offset, ties);
        }
    }
}

fn fill_cell_background(target: &mut [u8], width: usize, cell_x: u16, cell_y: u16, color: (u8, u8, u8, u8)) {
    let base_x = usize::from(cell_x) * usize::from(CELL_WIDTH);
    let base_y = usize::from(cell_y) * usize::from(CELL_HEIGHT);
    for py in 0..usize::from(CELL_HEIGHT) {
        for px in 0..usize::from(CELL_WIDTH) {
            set_pixel(target, width, base_x + px, base_y + py, color);
        }
    }
}

fn draw_glyph(target: &mut [u8], width: usize, cell_x: u16, cell_y: u16, ch: char, color: (u8, u8, u8, u8)) {
    let Some(bitmap) = glyph_bitmap(ch) else {
        return;
    };
    let base_x = usize::from(cell_x) * usize::from(CELL_WIDTH);
    let base_y = usize::from(cell_y) * usize::from(CELL_HEIGHT);
    for (row_idx, row_bits) in bitmap.iter().copied().enumerate() {
        for dy in 0..2 {
            let py = base_y + row_idx * 2 + dy;
            for col_idx in 0..8 {
                if (row_bits >> col_idx) & 1 == 1 {
                    set_pixel(target, width, base_x + col_idx, py, color);
                }
            }
        }
    }
}

fn draw_scaled_glyph(
    target: &mut [u8],
    width: usize,
    origin_x: usize,
    origin_y: usize,
    ch: char,
    scale: usize,
    color: (u8, u8, u8, u8),
) {
    let Some(bitmap) = glyph_bitmap(ch) else {
        return;
    };
    for (row_idx, row_bits) in bitmap.iter().copied().enumerate() {
        for col_idx in 0..8usize {
            if (row_bits >> col_idx) & 1 == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        set_pixel(
                            target,
                            width,
                            origin_x + col_idx * scale + dx,
                            origin_y + row_idx * scale + dy,
                            color,
                        );
                    }
                }
            }
        }
    }
}

fn draw_centered_glyph(
    target: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    rect_w: usize,
    rect_h: usize,
    ch: char,
    color: (u8, u8, u8, u8),
) {
    let Some(bitmap) = glyph_bitmap(ch) else {
        return;
    };
    let scale = (rect_w.min(rect_h) / 8).clamp(1, 2);
    let glyph_w = 8 * scale;
    let glyph_h = 8 * scale;
    let origin_x = x + rect_w.saturating_sub(glyph_w) / 2;
    let origin_y = y + rect_h.saturating_sub(glyph_h) / 2;
    for (row_idx, row_bits) in bitmap.iter().copied().enumerate() {
        for col_idx in 0..8usize {
            if (row_bits >> col_idx) & 1 == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        set_pixel(
                            target,
                            width,
                            origin_x + col_idx * scale + dx,
                            origin_y + row_idx * scale + dy,
                            color,
                        );
                    }
                }
            }
        }
    }
}

fn glyph_bitmap(ch: char) -> Option<[u8; 8]> {
    BASIC_FONTS
        .get(ch)
        .or_else(|| LATIN_FONTS.get(ch))
        .or_else(|| BOX_FONTS.get(ch))
        .or_else(|| BLOCK_FONTS.get(ch))
        .or_else(|| MISC_FONTS.get(ch))
        .or_else(|| BASIC_FONTS.get('?'))
}

fn set_pixel(target: &mut [u8], width: usize, x: usize, y: usize, color: (u8, u8, u8, u8)) {
    let index = (y * width + x) * 4;
    if index + 3 >= target.len() {
        return;
    }
    target[index] = color.0;
    target[index + 1] = color.1;
    target[index + 2] = color.2;
    target[index + 3] = color.3;
}

fn color_to_rgba(color: Color, fallback: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
    match color {
        Color::Reset => fallback,
        Color::Black => (0, 0, 0, 255),
        Color::Red => (128, 0, 0, 255),
        Color::Green => (0, 128, 0, 255),
        Color::Yellow => (128, 128, 0, 255),
        Color::Blue => (0, 0, 128, 255),
        Color::Magenta => (128, 0, 128, 255),
        Color::Cyan => (0, 128, 128, 255),
        Color::Gray => (192, 192, 192, 255),
        Color::DarkGray => (128, 128, 128, 255),
        Color::LightRed => (255, 0, 0, 255),
        Color::LightGreen => (0, 255, 0, 255),
        Color::LightYellow => (255, 255, 0, 255),
        Color::LightBlue => (0, 0, 255, 255),
        Color::LightMagenta => (255, 0, 255, 255),
        Color::LightCyan => (0, 255, 255, 255),
        Color::White => (255, 255, 255, 255),
        Color::Rgb(r, g, b) => (r, g, b, 255),
        Color::Indexed(index) => indexed_color(index),
    }
}

fn indexed_color(index: u8) -> (u8, u8, u8, u8) {
    if index < 16 {
        const ANSI: [(u8, u8, u8); 16] = [
            (0, 0, 0),
            (128, 0, 0),
            (0, 128, 0),
            (128, 128, 0),
            (0, 0, 128),
            (128, 0, 128),
            (0, 128, 128),
            (192, 192, 192),
            (128, 128, 128),
            (255, 0, 0),
            (0, 255, 0),
            (255, 255, 0),
            (0, 0, 255),
            (255, 0, 255),
            (0, 255, 255),
            (255, 255, 255),
        ];
        let (r, g, b) = ANSI[index as usize];
        return (r, g, b, 255);
    }

    if index >= 232 {
        let shade = 8 + (index - 232) * 10;
        return (shade, shade, shade, 255);
    }

    let normalized = index - 16;
    let r = normalized / 36;
    let g = (normalized % 36) / 6;
    let b = normalized % 6;
    let convert = |v: u8| if v == 0 { 0 } else { v * 40 + 55 };
    (convert(r), convert(g), convert(b), 255)
}

fn fill_rect(
    target: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    rect_w: usize,
    rect_h: usize,
    color: (u8, u8, u8, u8),
) {
    for py in y..y.saturating_add(rect_h) {
        for px in x..x.saturating_add(rect_w) {
            set_pixel(target, width, px, py, color);
        }
    }
}

fn shade_pixels(target: &mut [u8], tint: (u8, u8, u8, u8), alpha: u8) {
    if alpha == 0 {
        return;
    }
    for pixel in target.chunks_exact_mut(4) {
        let base = (pixel[0], pixel[1], pixel[2], pixel[3]);
        let shaded = blend_color(base, tint, alpha);
        pixel[0] = shaded.0;
        pixel[1] = shaded.1;
        pixel[2] = shaded.2;
        pixel[3] = shaded.3;
    }
}

fn draw_rect_outline(
    target: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    rect_w: usize,
    rect_h: usize,
    color: (u8, u8, u8, u8),
) {
    if rect_w == 0 || rect_h == 0 {
        return;
    }
    draw_hline(target, width, x, x + rect_w.saturating_sub(1), y, color);
    draw_hline(
        target,
        width,
        x,
        x + rect_w.saturating_sub(1),
        y + rect_h.saturating_sub(1),
        color,
    );
    draw_vline(target, width, x, y, y + rect_h.saturating_sub(1), color);
    draw_vline(
        target,
        width,
        x + rect_w.saturating_sub(1),
        y,
        y + rect_h.saturating_sub(1),
        color,
    );
}

fn draw_hline(
    target: &mut [u8],
    width: usize,
    x0: usize,
    x1: usize,
    y: usize,
    color: (u8, u8, u8, u8),
) {
    for x in x0..=x1 {
        set_pixel(target, width, x, y, color);
    }
}

fn draw_vline(
    target: &mut [u8],
    width: usize,
    x: usize,
    y0: usize,
    y1: usize,
    color: (u8, u8, u8, u8),
) {
    for y in y0..=y1 {
        set_pixel(target, width, x, y, color);
    }
}

fn blend_color(base: (u8, u8, u8, u8), tint: (u8, u8, u8, u8), alpha: u8) -> (u8, u8, u8, u8) {
    let alpha = u16::from(alpha);
    let inv_alpha = 255 - alpha;
    (
        ((u16::from(tint.0) * alpha + u16::from(base.0) * inv_alpha) / 255) as u8,
        ((u16::from(tint.1) * alpha + u16::from(base.1) * inv_alpha) / 255) as u8,
        ((u16::from(tint.2) * alpha + u16::from(base.2) * inv_alpha) / 255) as u8,
        255,
    )
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 in_pos;
    attribute vec2 in_uv;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(in_pos, 0, 1);
        texcoord = in_uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;

    uniform sampler2D tex;

    void main() {
        gl_FragColor = texture2D(tex, texcoord);
    }"#;

    pub const METAL: &str = r#"
    #include <metal_stdlib>

    using namespace metal;

    struct Vertex
    {
        float2 in_pos   [[attribute(0)]];
        float2 in_uv    [[attribute(1)]];
    };

    struct RasterizerData
    {
        float4 position [[position]];
        float2 uv       [[user(locn0)]];
    };

    vertex RasterizerData vertexShader(Vertex v [[stage_in]])
    {
        RasterizerData out;
        out.position = float4(v.in_pos.xy, 0.0, 1.0);
        out.uv = v.in_uv;
        return out;
    }

    fragment float4 fragmentShader(RasterizerData in [[stage_in]], texture2d<float> tex [[texture(0)]], sampler texSmplr [[sampler(0)]])
    {
        return tex.sample(texSmplr, in.uv);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout { uniforms: vec![] },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::game::minimap::tile_at_render_area_click;
    use ratatui::layout::Rect;

    #[test]
    fn square_minimap_layout_centers_square_grid() {
        let area = ClickArea {
            x: 4,
            y: 3,
            width: 16,
            height: 8,
        };

        let layout = square_minimap_layout(area, 32, 32).expect("layout should exist");

        assert_eq!(layout.tile_size, 4);
        assert_eq!(layout.grid_w, 32);
        assert_eq!(layout.grid_h, 32);
        assert_eq!(layout.x, 32);
        assert_eq!(layout.y, 48);
    }

    #[test]
    fn square_layout_tile_at_pixel_maps_back_to_expected_tile() {
        let area = ClickArea {
            x: 1,
            y: 2,
            width: 16,
            height: 8,
        };
        let layout = square_minimap_layout(area, 32, 32).expect("layout should exist");

        let tile = square_layout_tile_at_pixel(layout, 32, 32, layout.x + 10 * 4 + 2, layout.y + 7 * 4 + 1);

        assert_eq!(tile, Some((10, 7)));
    }

    #[test]
    fn synthetic_minimap_click_matches_shared_terminal_mapping() {
        let area = ClickArea {
            x: 2,
            y: 3,
            width: 20,
            height: 10,
        };

        for &(tile_x, tile_y) in &[(0, 0), (5, 4), (9, 9)] {
            let col = synthetic_minimap_col(area, 10, tile_x);
            let row = synthetic_minimap_row(area, 10, tile_y);
            let mapped = tile_at_render_area_click(
                Rect::new(area.x, area.y, area.width, area.height),
                10,
                10,
                col,
                row,
            );
            assert_eq!(mapped, Some((tile_x, tile_y)));
        }
    }

    #[test]
    fn square_map_layout_centers_viewport_grid() {
        let area = ClickArea {
            x: 10,
            y: 5,
            width: 20,
            height: 10,
        };

        let layout = square_map_layout(area, 8, 6).expect("layout should exist");

        assert_eq!(layout.tile_size, 20);
        assert_eq!(layout.grid_w, 8);
        assert_eq!(layout.grid_h, 6);
        assert_eq!(layout.x, 80);
        assert_eq!(layout.y, 100);
    }
}
