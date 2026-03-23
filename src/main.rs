mod app;
mod audio;
mod core;
mod game_info;
mod textgen;
mod ui;

use std::{fs::File, io, time::{Duration, Instant}};

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

/// Rate-limits ticks to a fixed interval regardless of how often it is checked.
///
/// Accepts an explicit `now` so that tests can drive it with synthetic timestamps
/// instead of real wall-clock time.
struct TickGate {
    interval: Duration,
    last: Instant,
}

impl TickGate {
    fn new(interval: Duration) -> Self {
        Self { interval, last: Instant::now() }
    }

    /// Returns `true` (and resets the timer) if `now` is at least one interval
    /// past the last tick; otherwise returns `false`.
    fn check(&mut self, now: Instant) -> bool {
        if now.duration_since(self.last) >= self.interval {
            self.last = now;
            true
        } else {
            false
        }
    }
}

fn run_loop(renderer: &mut dyn Renderer) -> io::Result<()> {
    let mut app = app::AppState::new();
    app.attach_engine_channel();
    let mut tick_gate = TickGate::new(Duration::from_millis(16));

    loop {
        if let Err(e) = renderer.render(&mut app) {
            log::error!("[ui] Render failed: {e}");
            return Err(e);
        }

        // Block until at least one event arrives (or timeout for tick)
        if event::poll(Duration::from_millis(16))? {
            loop {
                let event = event::read()?;
                let ui_event = app::input::terminal_ui_event(&event);
                app.on_event(&ui_event);
                let action = app::input::translate_terminal_event(event);
                app.on_action(action);
                if !app.running || !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        // Tick at a fixed rate regardless of how many input events arrived.
        // Without this gate, fast mouse movement shortens the loop and causes
        // time-dependent state (e.g. the news ticker scroll) to advance faster.
        if tick_gate.check(Instant::now()) {
            app.on_tick();
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_gate_does_not_fire_before_interval() {
        let mut gate = TickGate::new(Duration::from_millis(16));
        let t0 = gate.last;
        assert!(!gate.check(t0));
        assert!(!gate.check(t0 + Duration::from_millis(5)));
        assert!(!gate.check(t0 + Duration::from_millis(15)));
    }

    #[test]
    fn tick_gate_fires_once_at_interval_then_resets() {
        let mut gate = TickGate::new(Duration::from_millis(16));
        let t0 = gate.last;

        assert!(gate.check(t0 + Duration::from_millis(16)));
        // Immediately after firing the timer resets; sub-interval checks must not fire.
        assert!(!gate.check(t0 + Duration::from_millis(17)));
        assert!(!gate.check(t0 + Duration::from_millis(31)));
        // One full interval past the last fire: fires again.
        assert!(gate.check(t0 + Duration::from_millis(32)));
    }

    #[test]
    fn tick_gate_suppresses_burst_of_rapid_checks() {
        // Regression: fast mouse movement must not cause extra ticks.
        let mut gate = TickGate::new(Duration::from_millis(16));
        let t0 = gate.last;
        let mut tick_count = 0u32;
        // Simulate 200 events arriving within 1 ms (well under the 16 ms interval).
        for i in 0..200u64 {
            if gate.check(t0 + Duration::from_micros(i * 5)) {
                tick_count += 1;
            }
        }
        assert_eq!(tick_count, 0, "burst of rapid events must not trigger any tick");
    }
}
