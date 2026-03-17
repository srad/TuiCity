# TuiCity 2000

A terminal-based city building simulation inspired by SimCity 2000, written in Rust using `ratatui`.

`tuicity2` features a decoupled, multi-threaded architecture where the simulation engine runs independently of the frontend renderer, allowing for smooth gameplay and a robust platform for future expansion.

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT-green)

---

## Features

- **Multi-threaded Simulation**: The game logic runs on a dedicated background thread, ensuring the UI remains responsive even during complex calculations.
- **Deep Simulation Mechanics**:
    - **RCI Demand**: Dynamic demand based on ideal residential, commercial, and industrial ratios.
    - **Power Grid**: BFS-based power distribution logic. Buildings require power connectivity to develop.
    - **Road Access**: Manhattan-distance pathfinding for zone development.
    - **Economic Management**: Adjustable tax rates (0-20%) that directly impact city growth and demand.
- **Deep Simulation Systems**: Pollution diffusion, land value, crime, fire risk, and active disasters (fire spread, flooding, tornadoes) — all implemented as independent `SimSystem` plug-ins.
- **Interactive UI**:
    - Drop-down menu bar (System / Speed / Disasters / Windows) — fully mouse-clickable.
    - Full mouse support for building and dragging (Lines/Rectangles).
    - Minimap for quick navigation.
    - Real-time status bar showing city name, treasury, annualised income (+green/-red), population, and date.
    - Info panel with RCI demand bars, sparkline demand history, and per-tile pollution/land value/crime overlays.
    - Budget popup with tax-rate slider and 24-month treasury sparkline.
- **Save/Load System**: Persistence for your urban creations, with backwards-compatible save files.

---

## How to Play

### Installation
Ensure you have [Rust](https://www.rust-lang.org/) installed.
```bash
git clone https://github.com/yourusername/tuicity2.git
cd tuicity2
cargo run --release
```

### Controls
| Key | Action |
|-----|--------|
| `Arrows` / `Mouse` | Move cursor / navigate map |
| `Mouse drag` | Draw roads, power lines, zones in a line or rectangle |
| `F1` | Open / navigate the menu bar |
| `1`, `2`, `3` | Zone: Residential, Commercial, Industrial |
| `R` | Road tool |
| `L` | Rail tool |
| `P` | Power Line tool |
| `E` | Power Plant |
| `K` | Park |
| `S` | Police Station |
| `F` | Fire Station |
| `B` | Bulldoze |
| `?` | Inspect tool |
| `$` / `B` | Open Budget popup |
| `Space` | Pause / Unpause simulation |
| `Ctrl+S` | Save city |
| `Q` / `Ctrl+C` | Quit |
| `ESC` | Back / close popup / cancel drag |

---

## Architecture & Design

`tuicity2` is designed with a strict separation of concerns to allow for easy porting to other frontends (like WGPU or Pixels).

### 1. Modular Simulation Engine (`src/core/sim/system.rs`)
The simulation is built on a "Plug-and-Play" architecture:
- **SimSystem Trait**: Simulation logic is divided into independent systems (e.g., `PowerSystem`, `GrowthSystem`, `FinanceSystem`, `HistorySystem`).
- **Extensible**: New mechanics (like Pollution or Crime) can be added by implementing the `SimSystem` trait without touching core engine code.
- **Background Ticking**: All systems run on a dedicated thread, processing one "month" of simulation data at a time.

### 2. Multi-threading & Thread Safety
The application utilizes a **Producer-Consumer** model:
- **Simulation Thread**: Runs a loop that waits for commands via an `mpsc` channel and executes them. It holds the authoritative state inside an `Arc<RwLock<SimulationEngine>>`.
- **UI Thread**: Handles user input and rendering. It sends commands to the simulation thread and acquires a short-lived **Read Lock** on the engine to display the map and stats.

### 3. UI State Machine (`src/app/screens/mod.rs`)
The frontend uses a **State Machine** pattern implemented with a `Screen` trait:
- **Modular Logic**: Each screen (Start, New City, Load City, In-Game) is a self-contained struct implementing `on_action`, `on_tick`, and `render`.
- **Screen Stack**: `AppState` manages a stack of screens, allowing for nested menus and easy transitions (Push, Pop, Replace).
- **Decoupled Rendering**: The core game loop delegates rendering to the active screen, which in turn uses the `Renderer` abstraction.

### 4. Renderer Abstraction (`src/ui/mod.rs`)
The project uses a `Renderer` trait:
```rust
pub trait Renderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()>;
}
```
Currently, `TerminalRenderer` implements this using `ratatui`. This abstraction means the `main.rs` loop doesn't know it's in a terminal, making it trivial to swap the backend.

### 5. Visual Data & Widgets
The UI leverages `ratatui` widgets for enhanced clarity:
- **RCI Demand Bars**: Visual bar charts in the Info Panel showing real-time city needs.
- **Sparkline History**: 24-month demand trends in the Info Panel; treasury history in the Budget popup.
- **Annualised Income**: Status bar shows net income (taxes − yearly maintenance) in green or red, always up to date.
- **Drop-down Menu Bar** (`tui-menu`): System, Speed, Disasters, and Windows menus, navigable by keyboard or mouse click.

---

## For Developers

### Testing
We maintain a robust test suite for the core engine logic to prevent regressions in city growth or building mechanics.
```bash
cargo test
```

### Project Structure
- `src/core/`: The "Model". Simulation logic, map data, and engine commands.
- `src/ui/`: The "View". Layouts, widgets, and renderer implementations.
- `src/app/`: The "Controller". Input handling, camera management, and command generation.

---

## License
MIT License. See `LICENSE` for details.
