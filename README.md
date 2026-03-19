# TuiCity 2000

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

Navigate with the **arrow keys** and confirm with **Enter**, or click an item with the mouse.

### Creating a New City

Selecting **New City** opens an interactive form with a live preview. You can use the keyboard to move between fields and adjust the generator, or click fields, sliders, and buttons directly:

| Field | Description | Default |
|-------|-------------|---------|
| **City Name** | Used as the save file prefix | — |
| **Seed (hex)** | A 16-character hex code that perfectly encodes the Water %, Trees %, and the random map noise seed. Pasting a code here will automatically snap the sliders to the exact percentages used. | random |
| **Water %** | Percentage of map tiles that are water | 20 |
| **Trees %** | Percentage of map tiles that are forest | 30 |

As you adjust the values, the preview map and the hex seed update in real-time. You can type into the name and seed fields, click the Water % and Trees % bars to jump to a value, click **Regenerate** to roll a fresh map, and activate **Start** with either **Enter** or a mouse click.

### Controls

| Key / Input | Action |
|-------------|--------|
| Arrow keys | Move cursor |
| Mouse move | Move cursor |
| Left click | Place / confirm action |
| Left click + drag on window title | Move that window |
| Left click on menu bar / popup | Open menus and activate menu items |
| Left click on minimap | Center the camera near the clicked location |
| Left click on toolbar / popup buttons | Select tools or close / confirm popup actions |
| Mouse scroll / horizontal wheel | Pan camera vertically or horizontally |
| Middle click + drag on map | Grab-pan the map |
| Left click on map scrollbars | Step, page, or drag the viewport |
| Space | Pause / resume simulation |
| `q` / `Ctrl+C` | Open the quit prompt |
| `Ctrl+S` | Save city |
| `Esc` | Cancel drag, close modal windows, or go back |
| `Tab` | Cycle map overlay modes (Normal → Power → Pollution → Land Value → Crime → Fire Risk) |
| `F1` | Open menu bar |
| `$` | Open budget window |

The **File** menu contains save/load/quit actions, the **Windows** menu can toggle the **Toolbox** panel and other utility windows, and the right-aligned **Help** and **About** items open modal reference windows.

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
| `e` | Coal Plant | $3000 | 4×4 | Opens the power picker and arms a coal plant by default |
| `g` | Gas Plant | $6000 | 4×4 | Arms a gas plant directly |
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

Press `Ctrl+S` at any time or use **File → Save City**. Save files are written to:

```
~/.tuicity2000/saves/{city_name}_{year}{month:02}.json
```

For example, a city named `Springfield` saved in March 2030 writes to `~/.tuicity2000/saves/Springfield_203003.json`.

Legacy saves under `~/.tuicity2/saves/` are still discovered by the load screen.

To resume, choose **Load City** from the main menu and select a save file from the list with the keyboard or mouse. If you are already inside a city, **File → Load City** first asks whether to save the current city before opening the load screen. The quit flow behaves the same way.

---

## Developer Guide

### Architecture Overview

TuiCity 2000 is structured around three main pillars:

**1. Screen Stack**

`AppState` manages a `Vec<Box<dyn Screen>>`. The shared screen contract (`Screen`, `ScreenTransition`, `AppContext`) lives in `src/app/screens/mod.rs`, while each concrete screen is implemented in its own module under `src/app/screens/`:

```rust
pub trait Screen {
    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition>;
    fn on_tick(&mut self, context: AppContext);
    fn build_view(&self, context: AppContext<'_>) -> ScreenView;
}
```

`ScreenTransition` variants (`Push`, `Replace`, `Pop`) drive navigation without coupling screens to each other.

**2. Background Simulation Thread**

The simulation runs in a dedicated OS thread managed by `core/engine.rs`. The main UI thread sends `SimCommand` messages over a `std::sync::mpsc` channel. The sim thread processes commands (step, pause, save, load, place tile) and writes results back into an `Arc<RwLock<Engine>>` that the UI reads for rendering.

**3. Frontend-Neutral Screen Views + UI Runtime**

Shared geometry and window helpers now live in `src/ui/runtime.rs` (`ClickArea`, `MapUiAreas`, `UiAreas`, centered/clamped window helpers). Each screen builds a frontend-neutral `ScreenView` in `src/ui/view.rs`, and the terminal renderer turns that view into `ratatui` output. In-game UI elements (map, tools panel, budget, inspect, tool chooser, help, about) are movable windows managed by `DesktopState`.

`InGameScreen` is also split into focused feature modules rather than one large implementation file:
- `ingame.rs` keeps the screen shell and top-level lifecycle
- `ingame_interaction.rs` owns map interaction, dragging, panning, and scrollbar logic
- `ingame_budget.rs` owns budget state, focus traversal, and tax editing
- `ingame_menu.rs` owns menu state and menu action mapping

### Module Tree

```
src/
├── main.rs                        Entry point; sets up terminal, spawns sim thread, runs event loop
├── app/
│   ├── mod.rs                     AppState and shared app shell
│   ├── camera.rs                  Camera (viewport offset, clamping, pan)
│   ├── input.rs                   Action enum; crossterm Event → Action translation
│   ├── line_drag.rs               LineDrag state (road/rail/power line placement)
│   ├── rect_drag.rs               RectDrag state (zone/bulldoze area placement)
│   ├── save.rs                    Save/load helpers (JSON ↔ file)
│   └── screens/
│       ├── mod.rs                 Screen trait, AppContext, ScreenTransition, re-exports
│       ├── start.rs               StartScreen + start-menu state/logic
│       ├── new_city.rs            NewCityScreen + generator form state/logic
│       ├── load_city.rs           LoadCityScreen + save-list state/logic
│       ├── ingame.rs              InGameScreen shell + top-level event lifecycle
│       ├── ingame_interaction.rs  Map interaction, drag flows, scrollbars, panning
│       ├── ingame_menu.rs         Menu model, actions, menu routing
│       ├── ingame_budget.rs       Budget window state, focus model, tax input logic
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
    ├── mod.rs                     Renderer selection + `ScreenView` dispatch
    ├── view.rs                    Frontend-neutral screen and in-game view models
    ├── frontends/
    │   ├── mod.rs                 Frontend entry points
    │   └── terminal/
    │       ├── mod.rs             Terminal frontend exports
    │       └── ingame.rs          Terminal renderer for the in-game desktop
    ├── runtime.rs                 Shared UI runtime helpers (hit areas, window clamp/center, focus cycling)
    ├── theme.rs                   Colour palette; tile glyphs; overlay tinting
    ├── game/
    │   ├── mod.rs                 Re-exports for game sub-widgets
    │   ├── map_view.rs            MapView widget (tile glyphs, overlays, scrollbars)
    │   ├── minimap.rs             MiniMap widget (scaled overview)
    │   ├── toolbar.rs             Tool palette widget with hotkey hints and costs
    │   ├── infopanel.rs           Tile info + demand bars
    │   ├── statusbar.rs           Status bar (tool, funds, date, population, pause)
    │   ├── budget.rs              Budget window widget (summary cards, sector taxes, forecast)
    │   ├── inspect_popup.rs       Inspect window content (tile stats, overlay values)
    │   ├── power_popup.rs         Power-plant picker popup widget
    │   └── disasters.rs           Disaster event notification rendering
    └── screens/
        ├── mod.rs                 Re-exports for screen renderers
        ├── start.rs               Start menu rendering
        ├── new_city.rs            New-city form rendering (name, seed, generator controls, preview map)
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
| `DesktopState` | `ui/runtime.rs` | Managed in-game windows, focus order, dragging, centering, clamping |
| `WindowState` | `ui/runtime.rs` | Position, size, visibility, modal/closable metadata for one window |
| `Screen` | `app/screens/mod.rs` | Trait implemented by every game state |
| `ScreenView` | `ui/view.rs` | Frontend-neutral view model returned by a screen |
| `ScreenTransition` | `app/screens/mod.rs` | Navigation result returned by `on_action` (Push, Replace, Pop) |

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

1. The `ratatui` event loop calls `TerminalRenderer::render(&mut app_state)`.
2. The active screen builds a frontend-neutral `ScreenView`.
3. `TerminalRenderer` matches that `ScreenView` and dispatches to the matching terminal renderer in `src/ui/screens/*` or `src/ui/frontends/terminal/ingame.rs`.
4. `DesktopState` computes the in-game window layout (menu bar, status bar, map, panel, budget, inspect, tool chooser, help, about), including centering, clamping, title bars, and close-button geometry.
5. `MapView` iterates visible tiles (camera offset + viewport size), picks colours and 2-cell sprites from `theme.rs`, and writes them into the buffer. To better match terminal font aspect ratios, **the map is rendered using double-width tile sprites** (each map tile maps to two horizontal terminal cells), with roads, rails, and power lines using dedicated left/right sprite pairs so they do not appear double-thick. When the map is larger than the viewport, dedicated DOS-style horizontal and vertical scrollbars are rendered beside it.
6. The panel window renders the toolbox, minimap, and tile-info section. Layout is computed from the managed panel window rect so it stays stable while dragging, and the toolbar itself is layout-driven rather than row-index driven. The minimap uses the same 2:1 horizontal sampling as the main map so its aspect ratio, viewport outline, and click-to-center behavior stay aligned with the primary map view.
7. Popup windows such as the tool chooser, budget window, inspect window, help window, and about window render through dedicated `ui/game/*` modules or shared terminal helpers. The menu bar is rendered last so it always appears on top.

### Adding a New Tool

1. **Add the variant** to the `Tool` enum in `src/core/tool.rs`.
2. **Define the tool metadata** in `Tool`: set `cost`, `target_tile` (or `None` for UI-only tools), `label`, `key_hint`, `can_place`, and `footprint`.
3. **Mark the interaction style** if needed by updating `uses_line_drag`, `uses_rect_drag`, or `uses_footprint_preview`.
4. **Bind the hotkey / selection path** in `InGameScreen::on_action` inside `src/app/screens/ingame.rs`.
5. **Let the engine place it** through the existing `EngineCommand::PlaceTool` / `PlaceLine` / `PlaceRect` flow in `src/core/engine.rs`.
6. **Expose the tool in the toolbox** in `src/ui/game/toolbar.rs`.
   Group new tools under the current chooser/button structure (`Zones`, `Power Plants`, `Buildings`, `Amusement`) or add a new chooser group if the category no longer fits the existing layout.

### Adding a New Sim System

1. **Implement `SimSystem`** — add a new struct in `src/core/sim/systems/mod.rs` (or a new submodule) with a `tick(&mut self, map: &mut Map, sim: &mut SimState)` method.
2. **Register it** in `run_monthly_sim` in `src/core/sim/systems/mod.rs` at the appropriate position.
3. **Add any new fields** the system needs to `SimState` in `src/core/sim/mod.rs` (derive `Serialize`/`Deserialize` to keep save compatibility).
4. **Optionally add an overlay** if the system produces per-tile data worth visualising: add a field to `TileOverlay` in `src/core/map/tile.rs` and handle the new overlay mode in `src/ui/game/map_view.rs` and `src/ui/theme.rs`.

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | Terminal UI framework — layout, widgets, rendering buffer |
| `crossterm` | 0.29 | Raw mode, mouse capture, cross-platform terminal control |
| `serde` | 1.0 | Serialisation traits |
| `serde_json` | 1.0 | JSON save file format |
| `rand` | 0.8 | Map generation RNG, disaster probability rolls |

---

*TuiCity 2000 is a hobby project. Contributions, bug reports, and feature requests are welcome via GitHub Issues.*
