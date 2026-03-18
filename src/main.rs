mod app;
mod core;
mod ui;

use std::{io, time::Duration, sync::mpsc};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ui::{Renderer, TerminalRenderer};

fn main() -> io::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let mut renderer = TerminalRenderer::new()?;
    let result = run(&mut renderer);

    // Always restore terminal
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    result
}

fn run(renderer: &mut dyn Renderer) -> io::Result<()> {
    let mut app = app::AppState::new();
    let (tx, rx) = mpsc::channel();
    app.cmd_tx = Some(tx);

    let engine_arc = app.engine.clone();
    
    std::thread::spawn(move || {
        loop {
            // Process all pending commands
            while let Ok(cmd) = rx.try_recv() {
                let mut engine = engine_arc.write().unwrap();
                let _ = engine.execute_command(cmd);
            }
            
            // Sleep a bit to prevent 100% CPU usage
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    loop {
        renderer.render(&mut app)?;

        if event::poll(Duration::from_millis(16))? { // ~60fps poll
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
