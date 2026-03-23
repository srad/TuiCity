mod app;
mod audio;
mod core;
mod game_info;
mod textgen;
mod ui;

use std::{fs::File, io, sync::mpsc, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use app::config::FrontendKind;
use simplelog::*;
use ui::{Renderer, TerminalRenderer};

fn main() -> io::Result<()> {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        File::create("tc2000.log").unwrap(),
    )])
    .unwrap();

    let frontend = app::config::get_frontend_kind();
    log::info!("[main] starting {}", frontend.label());
    run_terminal(frontend)?;
    Ok(())
}

fn run_terminal(frontend: FrontendKind) -> io::Result<()> {
    // Redirect stderr to NUL before entering the alternate screen.
    // All app logging goes to the file logger; this silences C libraries
    // (e.g. llama.cpp) that write directly to stderr via fprintf.
    silence_stderr();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let mut renderer = TerminalRenderer::new(frontend)?;
    log::info!("[main] Starting run_loop");
    let result = run_loop(&mut renderer);
    log::info!("[main] run_loop finished");

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    result
}

/// Redirect C stderr (fd 2) to NUL / /dev/null so that native libraries
/// cannot corrupt the terminal alternate screen.
fn silence_stderr() {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        extern "C" {
            fn _open_osfhandle(osfhandle: isize, flags: i32) -> i32;
            fn _dup2(fd: i32, fd2: i32) -> i32;
        }
        if let Ok(nul) = File::open("NUL") {
            unsafe {
                let fd = _open_osfhandle(nul.as_raw_handle() as isize, 0);
                if fd >= 0 {
                    _dup2(fd, 2);
                }
            }
            std::mem::forget(nul);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        if let Ok(nul) = File::open("/dev/null") {
            unsafe {
                libc::dup2(nul.as_raw_fd(), 2);
            }
        }
    }
}

fn run_loop(renderer: &mut dyn Renderer) -> io::Result<()> {
    let mut app = app::AppState::new();
    let (tx, rx) = mpsc::channel();
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

    loop {
        if let Err(e) = renderer.render(&mut app) {
            log::error!("[ui] Render failed: {e}");
            return Err(e);
        }

        // Block until at least one event arrives (or timeout for tick)
        if event::poll(Duration::from_millis(16))? {
            // Drain ALL pending events before next render
            loop {
                let event = event::read()?;
                app.on_event(&event);
                let action = app::input::translate_event(event);
                app.on_action(action);
                if !app.running || !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        // Always tick — animations and sim clock must not stall during input
        app.on_tick();

        if !app.running {
            break;
        }
    }

    Ok(())
}
