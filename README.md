# TuiCity2

A terminal-based city-building simulation written in Rust. Build roads, zone land, manage power grids, balance budgets, and survive disasters — all inside your terminal.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

---

## Table of Contents

- [Playing the Game](#playing-the-game)
  - [Prerequisites & Building](#prerequisites--building)
  - [Main Menu](#main-menu)
  - [Creating a New City](#creating-a-new-city)
  - [Controls](#controls)
  - [Tools](#tools)
  - [Tips](#tips)
  - [Save & Load](#save--load)
- [Developer Guide](#developer-guide)
  - [Architecture Overview](#architecture-overview)
  - [Module Tree](#module-tree)
  - [Key Data Types](#key-data-types)
  - [Simulation Pipeline](#simulation-pipeline)
  - [Rendering Pipeline](#rendering-pipeline)
  - [Adding a New Tool](#adding-a-new-tool)
  - [Adding a New Sim System](#adding-a-new-sim-system)
  - [Dependencies](#dependencies)

---

## Playing the Game

### Prerequisites & Building

- **Rust 1.70+** — install via [rustup.rs](https://rustup.rs)
- A terminal emulator with UTF-8 support and mouse input (Windows Terminal, iTerm2, kitty, etc.)

```bash
# Clone the repo
git clone https://github.com/yourname/tuicity2
cd tuicity2

# Debug build
cargo build

# Release build (recommended for gameplay)
cargo build --release
```

### Main Menu

Launch the game:

```bash
cargo run --release
```

The main menu offers three options:

| Option | Description |
|--------|-------------|
| **New City** | Create a procedurally generated map and start building |
| **Load City** | Resume a previously saved city |
| **Quit** | Exit the application |

Navigate with the **arrow keys** and confirm with **Enter**.

### Creating a New City

Selecting **New City** opens a form with four fields:

| Field | Description | Default |
|-------|-------------|---------|
| **City Name** | Used as the save file prefix | — |
| **Seed** | RNG seed for map generation (integer) | random |
| **Water %** | Percentage of map tiles that are water | 20 |
| **Trees %** | Percentage of map tiles that are forest | 30 |

Tab between fields, edit with the keyboard, and press **Enter** to generate and enter the city.

### Controls

| Key / Input | Action |
|-------------|--------|
| Arrow keys | Move cursor |
| Mouse move | Move cursor |
| Left click | Place / confirm action |
| Left click + drag on window title | Move that window |
| Mouse scroll | Pan camera vertically |
| Space | Pause / resume simulation |
| `q` / `Ctrl+C` | Quit |
| `Ctrl+S` | Save city |
| `Esc` | Cancel drag / go back |
| `Tab` | Cycle map overlay modes (Normal → Power → Pollution → Land Value → Crime → Fire Risk) |
| `F1` | Open menu bar |
| `$` | Open budget window |

### Tools

Select a tool by pressing its hotkey. For area tools, click and drag to define the region; release to place.

| Key | Tool | Cost | Footprint | Description |
|-----|------|------|-----------|-------------|
| `?` | Inspect | free | 1×1 | Show tile info (zone, building, land value, pollution, power) |
| `1` | Residential Zone | $100 | 1×1 drag | Mark land for housing; buildings grow automatically |
| `2` | Commercial Zone | $100 | 1×1 drag | Mark land for shops and offices |
| `3` | Industrial Zone | $100 | 1×1 drag | Mark land for factories; generates jobs and pollution |
| `r` | Road | $10 | 1×1 drag | Connect zones; required for growth |
| `l` | Rail | $20 | 1×1 drag | High-capacity transit; boosts nearby land value |
| `p` | Power Line | $5 | 1×1 drag | Carry electricity from plants to zones |
| `e` | Power Plant | $3000 | 4×4 | Generates power; BFS floods electricity outward |
| `k` | Park | $80 | 2×2 | Increases land value in surrounding tiles |
| `s` | Police Station | $500 | 3×3 | Reduces crime within radius 12 |
| `f` | Fire Department | $500 | 3×3 | Reduces fire risk within radius 12 |
| `b` | Bulldoze | $1 | 1×1 drag | Remove any tile contents |

### Tips

**Economy**
- Tax revenue comes from buildings, not zones. Zones must grow into buildings first — that requires roads, power, and positive demand.
- Keep residential, commercial, and industrial demand roughly balanced. Check the demand bars in the budget window (`$`).
- Maintenance costs accumulate every month. Overbuilding infrastructure before you have tax income will bankrupt a young city quickly.

**Power Grid**
- A single power plant covers a large area via BFS flood-fill, but power lines must form a connected path to every zone.
- Unpowered zones will not grow. Use the Inspect tool (`?`) to verify power status.

**Zoning Ratios**
- A healthy city runs approximately 50% residential, 12% commercial, 38% industrial by tile count.
- Too much industry drives down land value through pollution. Buffer industrial zones with roads or parks.

**Disaster Prep**
- Fire stations are the most cost-effective disaster defence. One station per residential cluster keeps fire spread contained.
- Tornadoes occur in month 3 (2% chance). Keep funds in reserve rather than spending everything in February.
- Flooding occurs in month 6 (10% chance). Avoid placing critical infrastructure immediately adjacent to water.

### Save & Load

Press `Ctrl+S` at any time to save. Save files are written to:

```
~/.tuicity2/saves/{city_name}_{year}{month:02}.json
```

For example, a city named `Springfield` saved in March 2030 writes to `~/.tuicity2/saves/Springfield_203003.json`.

To resume, choose **Load City** from the main menu and select a save file from the list.

---

## Developer Guide

### Architecture Overview

TuiCity2 is structured around three main pillars:

**1. Screen Stack**

`AppState` manages a `Vec<Box<dyn Screen>>`. Every game state (start menu, new-city form, load-city list, in-game view) is an independent struct implementing the `Screen` trait defined in `src/app/screens/mod.rs`:

```rust
pub trait Screen {
    fn on_action(&mut self, action: Action, context: AppContext) -> Option<Transition>;
    fn on_tick(&mut self, context: AppContext);
    fn render(&mut self, frame: &mut Frame, context: AppContext);
}
```

`Transition` variants (`Push`, `Replace`, `Pop`) drive navigation without coupling screens to each other.

**2. Background Simulation Thread**

The simulation runs in a dedicated OS thread managed by `core/engine.rs`. The main UI thread sends `SimCommand` messages over a `std::sync::mpsc` channel. The sim thread processes commands (step, pause, save, load, place tile) and writes results back into an `Arc<RwLock<Engine>>` that the UI reads for rendering.

**3. Floating Window UI**

In-game UI elements (map, tools panel, budget, inspect) are independently movable `FloatingWindow` instances stored on `InGameScreen`. Each window can be dragged by its title bar to any position; windows may extend partially off-screen. The renderer clips each window rect to the terminal buffer boundary before rendering.

### Module Tree

```
src/
├── main.rs                        Entry point; sets up terminal, spawns sim thread, runs event loop
├── app/
│   ├── mod.rs                     AppState, ClickArea, FloatingWindow, WindowDrag
│   ├── camera.rs                  Camera (viewport offset, clamping, pan)
│   ├── input.rs                   Action enum; crossterm Event → Action translation
│   ├── line_drag.rs               LineDrag state (road/rail/power line placement)
│   ├── rect_drag.rs               RectDrag state (zone/bulldoze area placement)
│   ├── save.rs                    Save/load helpers (JSON ↔ file)
│   └── screens/
│       └── mod.rs                 Screen trait + all screen structs:
│                                    StartScreen, NewCityScreen, LoadCityScreen, InGameScreen
├── core/
│   ├── mod.rs                     Re-exports
│   ├── engine.rs                  Engine struct; sim thread loop; SimCommand dispatch
│   ├── tool.rs                    Tool enum, placement logic, cost/footprint/key-hint
│   ├── tests.rs                   Integration tests for tool placement and finance
│   ├── map/
│   │   ├── mod.rs                 Map struct; get/set/overlay accessors; power BFS
│   │   ├── tile.rs                Tile + TileOverlay enums; name/is_building helpers
│   │   └── gen.rs                 Procedural map generation (noise, water, forest)
│   └── sim/
│       ├── mod.rs                 SimState; MaintenanceBreakdown; demand/history fields
│       ├── system.rs              SimSystem trait
│       ├── growth.rs              Zone → building growth logic
│       └── systems/
│           └── mod.rs             All SimSystem impls + run_monthly_sim orchestrator:
│                                    PowerSystem, PollutionSystem, LandValueSystem,
│                                    PoliceSystem, FireSystem, GrowthSystem,
│                                    FinanceSystem, HistorySystem,
│                                    FireSpreadSystem, FloodSystem, TornadoSystem
└── ui/
    ├── mod.rs                     TerminalRenderer; render_game_v2 (floating window layout)
    ├── theme.rs                   Colour palette; tile glyphs; overlay tinting
    ├── game/
    │   ├── mod.rs                 Re-exports for game sub-widgets
    │   ├── map_view.rs            MapView widget (tile glyphs, overlays, scrollbars)
    │   ├── minimap.rs             MiniMap widget (scaled overview)
    │   ├── toolbar.rs             Tool button list with hotkey hints and costs
    │   ├── infopanel.rs           Tile info + demand bars
    │   ├── statusbar.rs           Status bar (tool, funds, date, population, pause)
    │   ├── budget.rs              Budget window content (treasury, tax, maintenance, sparkline)
    │   ├── inspect_popup.rs       Inspect window content (tile stats, overlay values)
    │   └── disasters.rs           Disaster event notification rendering
    └── screens/
        ├── mod.rs                 Re-exports for screen renderers
        ├── start.rs               Start menu rendering
        ├── new_city.rs            New-city form rendering (name, seed, sliders, preview map)
        └── load_city.rs           Load-city file list rendering
```

### Key Data Types

| Type | Location | Description |
|------|----------|-------------|
| `Tool` | `core/tool.rs` | Enum of all 12 tools; carries placement logic, cost, footprint, and key hint |
| `Tile` | `core/map/tile.rs` | Single map cell type: terrain, zone, or building variant |
| `TileOverlay` | `core/map/tile.rs` | Per-tile overlay data: powered, on_fire, pollution, land_value, crime, fire_risk |
| `Map` | `core/map/mod.rs` | Grid of tiles + overlays; width/height; BFS power-grid flood-fill |
| `SimState` | `core/sim/mod.rs` | Full simulation snapshot: treasury, date, population, demand, history, last_breakdown |
| `Camera` | `app/camera.rs` | Viewport offset; clamping; mouse-pan delta |
| `FloatingWindow` | `app/mod.rs` | Position + size of a movable window; title-bar hit-test |
| `WindowDrag` | `app/mod.rs` | Tracks which window is being dragged and the grab offset |
| `Screen` | `app/screens/mod.rs` | Trait implemented by every game state |
| `Transition` | `app/screens/mod.rs` | Navigation result returned by `on_action` (Push, Replace, Pop) |

### Simulation Pipeline

`run_monthly_sim` in `core/sim/systems/mod.rs` runs these steps in order each simulated month:

| # | System | What it does |
|---|--------|--------------|
| 1 | `PowerSystem` | BFS flood-fill from every power plant; marks tiles as powered |
| 2 | `PollutionSystem` | Radial diffusion from industrial tiles; attenuates with distance; parks scrub nearby pollution |
| 3 | `LandValueSystem` | Scores each tile: +water view, +parks, +hospitals; −pollution |
| 4 | `PoliceSystem` | Sets baseline crime per zone density; police stations suppress within radius 12 |
| 5 | `FireSystem` | Sets baseline fire risk per zone density; fire departments suppress within radius 12 |
| 6 | `GrowthSystem` | Converts zoned tiles to buildings when demand, power, and road access are satisfied |
| 7 | `FireSpreadSystem` | Rolls spontaneous ignition on high-risk buildings; spreads to neighbours; stations suppress; damaged buildings downgrade |
| 8 | `FloodSystem` | Month 6 only (10% chance): expands water tiles one cell outward |
| 9 | `TornadoSystem` | Month 3 only (2% chance): bulldozes a random 3-tile-wide path across the map |
| 10 | `FinanceSystem` | Charges monthly maintenance; collects annual tax (month 1); recalculates demand ratios |
| 11 | `HistorySystem` | Appends demand and treasury to 24-month rolling buffers |

### Rendering Pipeline

1. The `ratatui` event loop calls `TerminalRenderer::render(frame, &app_state)`.
2. `TerminalRenderer` peeks at the top of the screen stack and calls `screen.render(frame, context)`.
3. For `InGameScreen`, rendering is delegated to `render_game_v2` in `src/ui/mod.rs`.
4. `render_game_v2` manages four `FloatingWindow` instances (map, panel, budget, inspect):
   - Each window is clamped so its title bar stays on-screen and neither dimension writes outside the buffer.
   - Windows are rendered in back-to-front order: background → map → panel → budget → inspect → menu bar.
   - `Clear` is rendered before each window to erase whatever is behind it.
5. `MapView` iterates visible tiles (camera offset + viewport size), picks a glyph and colour from `theme.rs`, and writes `Cell` values into the buffer. Scrollbar overlays are drawn on the last column and row if the map is larger than the viewport.
6. The panel window renders toolbar buttons, a minimap, and a tile-info section. Layout is computed from the window's nominal height (not its clipped height), so sub-widget proportions stay constant when the window is dragged toward the screen edge.
7. The menu bar (`tui-menu`) is rendered last so it always appears on top.

### Adding a New Tool

1. **Add the variant** to the `Tool` enum in `src/core/tool.rs`.
2. **Implement placement** in the `Tool::place` match arm — return the cost and any `SimCommand`s needed.
3. **Set the cost** in `Tool::cost`.
4. **Set the footprint** in `Tool::footprint` (`Point`, `Rect`, `Line`, or `Centered(w, h)`).
5. **Set the key hint** in `Tool::key_hint` and **bind the hotkey** in `InGameScreen::on_action` inside `src/app/screens/mod.rs`.
6. **Add the tool to a group** in `TOOL_GROUPS` in `src/ui/game/toolbar.rs` so it appears in the panel.

### Adding a New Sim System

1. **Implement `SimSystem`** — add a new struct in `src/core/sim/systems/mod.rs` (or a new submodule) with a `tick(&mut self, map: &mut Map, sim: &mut SimState)` method.
2. **Register it** in `run_monthly_sim` in `src/core/sim/systems/mod.rs` at the appropriate position.
3. **Add any new fields** the system needs to `SimState` in `src/core/sim/mod.rs` (derive `Serialize`/`Deserialize` to keep save compatibility).
4. **Optionally add an overlay** if the system produces per-tile data worth visualising: add a field to `TileOverlay` in `src/core/map/tile.rs` and handle the new overlay mode in `src/ui/game/map_view.rs` and `src/ui/theme.rs`.

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | Terminal UI framework — layout, widgets, rendering buffer |
| `crossterm` | 0.28 | Raw mode, mouse capture, cross-platform terminal control |
| `serde` | 1.0 | Serialisation traits |
| `serde_json` | 1.0 | JSON save file format |
| `rand` | 0.8 | Map generation RNG, disaster probability rolls |
| `tui-menu` | 0.3 | Dropdown menu bar widget (F1 menu) |

---

*TuiCity2 is a hobby project. Contributions, bug reports, and feature requests are welcome via GitHub Issues.*
