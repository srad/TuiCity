mod font;
mod input;
mod paint;
mod tiles;

use std::{
    io,
    num::NonZeroU32,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use softbuffer::{Context, Surface};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::ModifiersState,
    window::WindowBuilder,
};

use crate::{
    app::{screens::InGameScreen, AppState},
    core::engine::EngineCommand,
    ui::view::ScreenView,
};

use font::cell_scale;
use input::{pixels_to_cell, translate_key_event, translate_scroll};

/// Entry point for the pixel GUI frontend.
pub fn run() -> io::Result<()> {
    let event_loop =
        EventLoop::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("TuiCity 2000")
            .with_inner_size(LogicalSize::new(1280u32, 800u32))
            .with_min_inner_size(LogicalSize::new(400u32, 300u32))
            .build(&event_loop)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?,
    );

    let context = Context::new(window.clone())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut surface = Surface::new(&context, window.clone())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut app = AppState::new();
    let (tx, rx) = mpsc::channel::<EngineCommand>();
    app.cmd_tx = Some(tx);

    let engine_arc = app.engine.clone();
    std::thread::spawn(move || {
        while let Ok(cmd) = rx.recv() {
            let mut engine = engine_arc.write().unwrap();
            let _ = engine.execute_command(cmd);
            // Drain any additional queued commands while we hold the lock
            while let Ok(cmd) = rx.try_recv() {
                let _ = engine.execute_command(cmd);
            }
        }
    });

    let mut cursor_pos: (f64, f64) = (0.0, 0.0);
    let mut mouse_down = false;
    let mut modifiers = ModifiersState::default();
    let mut scale: u32 = cell_scale(window.scale_factor());

    const FRAME_DT: Duration = Duration::from_millis(33); // ~30 FPS
    let mut last_frame = Instant::now();
    let mut needs_redraw = true;

    event_loop
        .run(move |event, elwt| {
            let now = Instant::now();
            let next_frame = last_frame + FRAME_DT;
            if now >= next_frame {
                elwt.set_control_flow(ControlFlow::Poll);
            } else {
                elwt.set_control_flow(ControlFlow::WaitUntil(next_frame));
            }

            match event {
                Event::AboutToWait => {
                    let now = Instant::now();
                    if now >= last_frame + FRAME_DT {
                        app.on_tick();
                        if !app.running {
                            elwt.exit();
                            return;
                        }
                        if needs_redraw {
                            window.request_redraw();
                        }
                        last_frame = now;
                    }
                }

                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }

                        WindowEvent::RedrawRequested => {
                            let phys = window.inner_size();
                            let (w, h) = (phys.width, phys.height);
                            if w == 0 || h == 0 {
                                return;
                            }

                            if let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
                                surface.resize(nw, nh).ok();
                            }

                            if let Ok(mut sb_buf) = surface.buffer_mut() {
                                let view = build_view(&mut app);
                                let ingame = if matches!(view, ScreenView::InGame(_)) {
                                    app.screens
                                        .last_mut()
                                        .and_then(|s| s.as_any_mut().downcast_mut::<InGameScreen>())
                                } else {
                                    None
                                };
                                paint::paint_frame(&mut sb_buf, w, h, scale, &view, ingame);
                                sb_buf.present().ok();
                            }
                            needs_redraw = true; // always redraw next frame for animations
                        }

                        WindowEvent::ModifiersChanged(mods) => {
                            modifiers = mods.state();
                        }

                        WindowEvent::KeyboardInput { event: ke, .. } => {
                            let action = translate_key_event(&ke, modifiers);
                            app.on_action(action);
                        }

                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            scale = cell_scale(scale_factor);
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            cursor_pos = (position.x, position.y);
                            let (col, row) = pixels_to_cell(position.x, position.y, scale);
                            if mouse_down {
                                app.on_action(crate::app::input::Action::MouseDrag { col, row });
                            } else {
                                app.on_action(crate::app::input::Action::MouseMove { col, row });
                            }
                        }

                        WindowEvent::MouseInput { state, button, .. } => {
                            let (col, row) = pixels_to_cell(cursor_pos.0, cursor_pos.1, scale);
                            match (button, state) {
                                (MouseButton::Left, ElementState::Pressed) => {
                                    mouse_down = true;
                                    app.on_action(crate::app::input::Action::MouseClick {
                                        col,
                                        row,
                                    });
                                }
                                (MouseButton::Left, ElementState::Released) => {
                                    mouse_down = false;
                                    app.on_action(crate::app::input::Action::MouseUp { col, row });
                                }
                                (MouseButton::Middle, ElementState::Pressed) => {
                                    app.on_action(crate::app::input::Action::MouseMiddleDown {
                                        col,
                                        row,
                                    });
                                }
                                (MouseButton::Middle, ElementState::Released) => {
                                    app.on_action(crate::app::input::Action::MouseMiddleUp);
                                }
                                _ => {}
                            }
                        }

                        WindowEvent::MouseWheel { delta, .. } => {
                            let action = translate_scroll(&delta);
                            app.on_action(action);
                        }

                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

/// Build a `ScreenView` snapshot without keeping a long-lived borrow on the screen.
fn build_view(app: &mut AppState) -> ScreenView {
    let context = crate::app::screens::AppContext {
        engine: &app.engine,
        cmd_tx: &app.cmd_tx,
        textgen: &app.textgen,
    };
    app.screens
        .last_mut()
        .map(|s| s.build_view(context))
        .unwrap_or_else(|| {
            ScreenView::Start(crate::ui::view::StartViewModel {
                selected: 0,
                options: vec![],
            })
        })
}
