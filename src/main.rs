mod app;
mod audio;
mod core;
mod game_info;
mod ui;

use std::{io, sync::mpsc, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use app::config::FrontendKind;
use ui::{Renderer, TerminalRenderer};

fn main() -> io::Result<()> {
    match app::config::get_frontend_kind() {
        FrontendKind::PixelsGui => ui::frontends::pixels_winit::run(),
        FrontendKind::Terminal => run_terminal(),
    }?;
    Ok(())
}

fn run_terminal() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let mut renderer = TerminalRenderer::new()?;
    let result = run_loop(&mut renderer);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    result
}

fn run_loop(renderer: &mut dyn Renderer) -> io::Result<()> {
    let mut app = app::AppState::new();
    let (tx, rx) = mpsc::channel();
    app.cmd_tx = Some(tx);

    let engine_arc = app.engine.clone();

    std::thread::spawn(move || {
        loop {
            while let Ok(cmd) = rx.try_recv() {
                let mut engine = engine_arc.write().unwrap();
                let _ = engine.execute_command(cmd);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    loop {
        renderer.render(&mut app)?;

        if event::poll(Duration::from_millis(16))? {
            let event = event::read()?;
            app.on_event(&event);
            let action = app::input::translate_event(event);
            app.on_action(action);
        } else {
            app.on_tick();
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}
