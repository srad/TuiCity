# TuiCity 2000

A terminal-based city-building simulation written in Rust. Build layered cities with roads, power, water, zoning, transit, budgets, and disasters, all inside your terminal.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

---

## Table of Contents

- [Playing the Game](#playing-the-game)
  - [Prerequisites & Building](#prerequisites--building)
  - [Main Menu](#main-menu)
  - [Music](#music)
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

The main menu offers four options:

| Option | Description |
|--------|-------------|
| **Load Existing City** | Resume a previously saved city |
| **Create New City** | Create a procedurally generated map and start building |
| **Settings** | Open configuration screens such as theme selection |
| **Quit** | Exit the application |

Navigate with the **arrow keys** and confirm with **Enter**, or click an item with the mouse.

### Music

- The soundtrack working set lives in `assets/music`.
- The game automatically scans the `assets/music` folder for `.mp3` files at startup.
- All discovered tracks are played in a random order without repeating until the entire playlist has finished, at which point the playlist is reshuffled and restarted automatically.
- Playback runs in a dedicated background thread using the cross-platform `rodio` crate (with only the MP3 decoding feature enabled to keep the build size minimal).
- The music can be enabled or disabled at any time via the **Start Screen** or in-game by going to **File -> Toggle Music** (shortcut `M`). Your preference is saved persistently.
- See `assets/music/LMMS_SURGE_WORKFLOW.md` for the current authoring workflow.
- To add more in-game music, simply drop the `.mp3` exports into the `assets/music` folder. No code changes are required for the game to pick them up.

### Creating a New City

Selecting **New City** opens an interactive form with a live preview. You can use the keyboard to move between fields and adjust the generator, or click fields, sliders, and buttons directly:

| Field | Description | Default |
|-------|-------------|---------|
| **City Name** | Used as the save file prefix | — |
| **Seed (hex)** | A 16-character hex code that perfectly encodes the Water %, Trees %, and the random map noise seed. Pasting a code here will automatically snap the sliders to the exact percentages used. | random |
| **Water %** | Percentage of map tiles that are water | 20 |
| **Trees %** | Percentage of map tiles that are forest | 30 |

As you adjust the values, the preview map and the hex seed update in real-time. You can type into the name and seed fields, use **Left/Right** on the focused Water % and Trees % sliders, click those bars to jump to a value, click **Regenerate** to roll a fresh map, and activate **Start** with either **Enter** or a mouse click.

### Controls

| Key / Input | Action |
|-------------|--------|
| Arrow keys | Move cursor |
| Mouse move | Move the map cursor and live placement preview |
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
| `u` | Toggle the active surface / underground layer |
| `Esc` | Cancel drag, close modal windows, or go back |
| `Tab` | Cycle map overlay modes (Normal → Power Grid → Water Service → Traffic → Pollution → Land Value → Crime → Fire Risk) |
| `F1` | Open menu bar |
| `b` / `B` / `$` | Open budget window |

The **File** menu contains save/load/quit and settings actions, the **Windows** menu can toggle the **Toolbox**, **Inspect**, **Statistics**, the active **View Layer**, and overlay modes, and the right-aligned **Help** and **About** items open modal reference windows. The status bar also exposes a persistent clickable **Surface / Underground** switch so layer state is always visible.

On the **Load City** screen, use **Arrow keys** to select a save, **Enter** to load it, and **d** to open the delete confirmation dialog for the selected city.

### Tools

Select a tool by pressing its hotkey or from the toolbox chooser. Moving the mouse over the map updates the active placement cursor and preview, including 1×1 ploppable tools such as **Water Pump**. For area tools, click and drag to define the region; release to place.

#### Common Hotkeys

| Key | Tool | Cost | Footprint | Description |
|-----|------|------|-----------|-------------|
| `?` | Inspect | free | 1×1 | Show tile info, overlays, trip status, and utility service |
| `1` | Residential Light Zone | $5 | 1×1 drag | Light-density housing zoning |
| `2` | Residential Dense Zone | $10 | 1×1 drag | Dense residential zoning; needs water to realize dense growth |
| `3` | Commercial Light Zone | $5 | 1×1 drag | Light commercial zoning |
| `4` | Commercial Dense Zone | $10 | 1×1 drag | Dense commercial zoning |
| `5` | Industrial Light Zone | $5 | 1×1 drag | Light industrial zoning |
| `6` | Industrial Dense Zone | $10 | 1×1 drag | Dense industrial zoning |
| `r` | Road | $10 | 1×1 drag | Local road access for trips and growth |
| `h` | Highway | $40 | 1×1 drag | Faster long-distance road network |
| `o` | Onramp | $25 | 1×1 drag | Connect highways back into the local road grid |
| `l` | Rail | $20 | 1×1 drag | Rail network for depot-based transit |
| `p` | Power Line | $5 | 1×1 drag | Surface electricity distribution; can run through zones |
| `w` | Water Pipe | $5 | 1×1 drag | Underground water distribution; selecting it switches to the underground layer |
| `m` | Subway Tunnel | $50 | 1×1 drag | Underground subway network |
| `e` | Coal Plant | $3000 | 4×4 | Early power generation |
| `g` | Gas Plant | $6000 | 4×4 | Higher-capacity power generation |
| `d` | Bus Depot | $250 | 2×2 | Enables bus trips on connected road networks |
| `t` | Rail Depot | $500 | 2×2 | Connects nearby lots to connected rail lines |
| `n` | Subway Station | $500 | 1×1 | Connects nearby lots to connected subway tunnels |
| `k` | Park | $80 | 2×2 | Raises nearby land value |
| `s` | Police Station | $500 | 3×3 | Reduces crime within radius 12 |
| `f` | Fire Department | $500 | 3×3 | Reduces fire risk within radius 12 |

#### Additional Toolbox Tools

| Tool | Cost | Footprint | Description |
|------|------|-----------|-------------|
| Bulldoze | $1 | 1×1 drag | Remove surface or underground contents depending on the active layer |
| Water Pump | $200 | 1×1 | Powered water production; stronger when placed next to water |
| Water Tower | $350 | 2×2 | Moderate powered water production |
| Water Treatment | $750 | 3×3 | High water production utility |
| Desalination | $1200 | 3×3 | Highest-capacity water production utility |

### Tips

**Economy**
- Tax revenue comes from buildings, not zones. Zones must first become transport-functional, powered, and supported by demand before they grow.
- Keep residential, commercial, and industrial demand roughly balanced. Check the demand bars in the budget window (`$`).
- Dense zoning is more powerful but more demanding. Dense zones need water service before they can realize dense buildings.
- Maintenance costs accumulate every month. Overbuilding infrastructure before you have tax income will bankrupt a young city quickly.

**Transport**
- Roads matter because the simulation now checks for usable trips, not just nearby pavement. A zone can be powered and still stall if it cannot reach what it needs.
- Highways extend the road network, but lots still need local road access. Use onramps to connect highways back into the city.
- Bus, rail, and subway service only work when depots or stations connect to the same underlying network.
- In the surface view, busy roads and highways now show ambient moving traffic markers so congestion is visible even without opening the traffic overlay.

**Power & Water**
- Power follows SC2000-style conduction: power lines, plants, and developed buildings conduct; empty zones receive service but do not relay it onward.
- Roads and power lines can cross on the same surface tile, SC2000-style; the shared tile still counts as both a road connection and a power connection.
- Water behaves similarly underground: pipes and developed buildings relay service, while empty zones do not chain utilities by themselves.
- The **Water Service** overlay is a surface coverage view. The **Underground** layer is a separate infrastructure view for pipes and subway tunnels, with faint roads and landmarks left visible for orientation.
- The **Power Grid** overlay now shows a subtle flow pulse on live power lines, while underground pipes and subway stations have their own low-key activity pulses in underground view.
- Power lines can be laid through zones and later be replaced by the building that grows there.
- Most plopped surface buildings can also be placed directly on bare power lines; the line is removed automatically as part of placement.

**Zoning Ratios**
- A healthy city runs approximately 50% residential, 12% commercial, 38% industrial by tile count.
- Too much industry drives down land value through pollution. Buffer industrial zones with roads or parks.

**Disaster Prep**
- Fire stations are the most cost-effective disaster defence. One station per residential cluster keeps fire spread contained.
- Active fires now flicker in the map view so burning tiles stand out immediately.
- Tornadoes occur in month 3 (2% chance). Keep funds in reserve rather than spending everything in February.
- Flooding occurs in month 6 (10% chance). Avoid placing critical infrastructure immediately adjacent to water.

### Save & Load

Press `Ctrl+S` at any time or use **File → Save City**. Save files are written to:

```
~/.tuicity2000/saves/{city_name}.tc2
```

For example, a city named `Springfield` saves to `~/.tuicity2000/saves/Springfield.tc2` and later saves overwrite that same file.

To resume, choose **Load City** from the main menu and select a save file from the list with the keyboard or mouse. Press `d` on the load screen to delete the selected save; deletion always goes through a confirmation dialog first. If you are already inside a city, **File → Load City** first asks whether to save the current city before opening the load screen. The quit flow behaves the same way.

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

The simulation runs in a dedicated OS thread managed by `core/engine.rs`. The main UI thread sends `EngineCommand` messages over a `std::sync::mpsc` channel. The sim thread processes commands (step, pause, save, load, place tile, replace state) and writes results back into a shared engine instance that the UI reads for rendering.

**3. Frontend-Neutral Screen Views + UI Runtime**

Shared geometry and window helpers now live in `src/ui/runtime.rs` (`ClickArea`, `MapUiAreas`, `UiAreas`, centered/clamped window helpers). Each screen builds a frontend-neutral `ScreenView` in `src/ui/view.rs`, and the terminal renderer turns that view into `ratatui` output. In-game UI elements (map, tools panel, budget, inspect, statistics, tool chooser, help, about) are movable windows managed by `DesktopState`.

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
│   ├── save.rs                    Binary `.tc2` save/load helpers
│   └── screens/
│       ├── mod.rs                 Screen trait, AppContext, ScreenTransition, re-exports
│       ├── start.rs               StartScreen + start-menu state/logic
│       ├── new_city.rs            NewCityScreen + generator form state/logic
│       ├── load_city.rs           LoadCityScreen + save-list state/logic
│       ├── settings.rs            Settings screen state/logic
│       ├── theme_settings.rs      Theme picker screen state/logic
│       ├── ingame.rs              InGameScreen shell + top-level event lifecycle
│       ├── ingame_interaction.rs  Map interaction, drag flows, scrollbars, panning
│       ├── ingame_menu.rs         Menu model, actions, menu routing
│       ├── ingame_budget.rs       Budget window state, focus model, tax input logic
├── core/
│   ├── mod.rs                     Re-exports
│   ├── engine.rs                  SimulationEngine; monthly system order; EngineCommand dispatch
│   ├── tool.rs                    Tool enum, placement logic, cost/footprint/key-hint
│   ├── tests.rs                   Integration tests for tool placement and finance
│   ├── map/
│   │   ├── mod.rs                 Layered map model; surface/underground composition
│   │   ├── tile.rs                Tile, transport, zoning, and overlay types
│   │   └── gen.rs                 Procedural map generation (noise, water, forest)
│   └── sim/
│       ├── mod.rs                 SimState; maintenance, utility, history, and transport summary fields
│       ├── system.rs              SimSystem trait
│       ├── growth.rs              Zone → building growth logic
│       ├── transport/mod.rs       Network cache, trip simulation, routing, and traffic accumulation
│       └── systems/
│           └── mod.rs             Core SimSystem implementations:
│                                    PowerSystem, WaterSystem, PollutionSystem,
│                                    LandValueSystem, PoliceSystem, FireSystem, GrowthSystem,
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
    │   ├── map_view.rs            MapView widget (tile glyphs, overlays, underground context, scrollbars)
    │   ├── minimap.rs             MiniMap widget (scaled overview)
    │   ├── toolbar.rs             Tool palette widget with hotkey hints and costs
    │   ├── infopanel.rs           Tile info + demand bars
    │   ├── statusbar.rs           Status bar (city stats, layer switch, messages, pause)
    │   ├── budget.rs              Budget window widget (summary cards, sector taxes, forecast)
    │   ├── inspect_popup.rs       Inspect window content (tile stats, overlay values)
    │   ├── power_popup.rs         Power-plant picker popup widget
    │   ├── statistics.rs          Statistics window widget (history charts)
    │   └── disasters.rs           Disaster event notification rendering
    └── screens/
        ├── mod.rs                 Re-exports for screen renderers
        ├── start.rs               Start menu rendering
        ├── new_city.rs            New-city form rendering (name, seed, generator controls, preview map)
        ├── load_city.rs           Load-city file list rendering
        ├── settings.rs            Settings screen rendering
        └── theme_settings.rs      Theme picker rendering
```

### Key Data Types

| Type | Location | Description |
|------|----------|-------------|
| `Tool` | `core/tool.rs` | Enum of all gameplay tools; explicit light/dense zoning, transport, power, water, and services |
| `Tile` | `core/map/tile.rs` | Visible tile variant for terrain, transport, zones, buildings, and utilities |
| `TileOverlay` | `core/map/tile.rs` | Per-tile overlay data: power, water, traffic, pollution, land value, crime, fire risk, trip diagnostics |
| `Map` | `core/map/mod.rs` | Layered map state: terrain, zoning, transport, power lines, underground utilities, occupants, overlays |
| `SimState` | `core/sim/mod.rs` | Full simulation snapshot: treasury, date, population, demand, history, utilities, transport summaries, RNG state |
| `Camera` | `app/camera.rs` | Viewport offset; clamping; mouse-pan delta |
| `DesktopState` | `ui/runtime.rs` | Managed in-game windows, focus order, dragging, centering, clamping |
| `WindowState` | `ui/runtime.rs` | Position, size, visibility, modal/closable metadata for one window |
| `Screen` | `app/screens/mod.rs` | Trait implemented by every game state |
| `ScreenView` | `ui/view.rs` | Frontend-neutral view model returned by a screen |
| `ScreenTransition` | `app/screens/mod.rs` | Navigation result returned by `on_action` (Push, Replace, Pop) |

### Simulation Pipeline

`SimulationEngine` in `core/engine.rs` runs these systems in order each simulated month:

| # | System | What it does |
|---|--------|--------------|
| 1 | `PowerSystem` | Ages plants, spreads power SC2000-style, and applies connected-network brownouts |
| 2 | `WaterSystem` | Produces water from powered utilities, spreads underground service, and applies connected-network shortages |
| 3 | `TransportSystem` | Builds network caches, simulates trips, chooses modes, and accumulates road traffic |
| 4 | `PollutionSystem` | Radial diffusion from industry and traffic; parks scrub nearby pollution |
| 5 | `LandValueSystem` | Scores each tile: +water view, +parks, +services; −pollution |
| 6 | `PoliceSystem` | Sets baseline crime per zone density; police stations suppress within radius 12 |
| 7 | `FireSystem` | Sets baseline fire risk per zone density; fire departments suppress within radius 12 |
| 8 | `GrowthSystem` | Converts zoned tiles to buildings when demand, utilities, and trip success are satisfied |
| 9 | `FireSpreadSystem` | Rolls spontaneous ignition on high-risk buildings; spreads to neighbours; damaged buildings downgrade |
| 10 | `FloodSystem` | Month 6 only (10% chance): expands water tiles one cell outward |
| 11 | `TornadoSystem` | Month 3 only (2% chance): bulldozes a random 3-tile-wide path across the map |
| 12 | `FinanceSystem` | Charges monthly maintenance; collects annual tax (month 1); recalculates demand ratios |
| 13 | `HistorySystem` | Appends demand, treasury, population, income, and power balance to rolling buffers |

### Rendering Pipeline

1. The `ratatui` event loop calls `TerminalRenderer::render(&mut app_state)`.
2. The active screen builds a frontend-neutral `ScreenView`.
3. `TerminalRenderer` matches that `ScreenView` and dispatches to the matching terminal renderer in `src/ui/screens/*` or `src/ui/frontends/terminal/ingame.rs`.
4. `DesktopState` computes the in-game window layout (menu bar, status bar, map, panel, budget, inspect, tool chooser, help, about), including centering, clamping, title bars, and close-button geometry.
5. `MapView` iterates visible tiles (camera offset + viewport size), picks colours and 2-cell sprites from `theme.rs`, and writes them into the buffer. To better match terminal font aspect ratios, **the map is rendered using double-width tile sprites** (each map tile maps to two horizontal terminal cells), with roads, rails, and power lines using dedicated left/right sprite pairs so they do not appear double-thick. Surface road traffic is also animated directly on those sprites using the current traffic overlay values, burning tiles use a small ASCII flicker cycle to make fires easier to spot, and utility overlays/layers add restrained pulses for active power lines, underground pipes, and subway stations. Water service remains a surface-oriented coverage overlay, while underground mode composites pipes/tunnels with ghosted roads and landmarks for orientation. When the map is larger than the viewport, dedicated DOS-style horizontal and vertical scrollbars are rendered beside it.
6. The panel window renders the toolbox, minimap, and tile-info section. Layout is computed from the managed panel window rect so it stays stable while dragging, and the toolbar itself is layout-driven rather than row-index driven. The minimap uses the same 2:1 horizontal sampling as the main map so its aspect ratio, viewport outline, and click-to-center behavior stay aligned with the primary map view.
7. Popup windows such as the tool chooser, budget window, inspect window, statistics window, help window, and about window render through dedicated `ui/game/*` modules or shared terminal helpers. The menu bar is rendered last so it always appears on top.

### Adding a New Tool

1. **Add the variant** to the `Tool` enum in `src/core/tool.rs`.
2. **Define the tool metadata** in `Tool`: set `cost`, `target_tile` (or `None` for UI-only tools), `label`, `key_hint`, `can_place`, and `footprint`.
3. **Mark the interaction style** if needed by updating `uses_line_drag`, `uses_rect_drag`, or `uses_footprint_preview`.
4. **Bind the hotkey / selection path** in `InGameScreen::on_action` inside `src/app/screens/ingame.rs`.
5. **Let the engine place it** through the existing `EngineCommand::PlaceTool` / `PlaceLine` / `PlaceRect` flow in `src/core/engine.rs`.
6. **Expose the tool in the toolbox** in `src/ui/game/toolbar.rs` and `src/ui/runtime.rs`.
   Group new tools under the current chooser/button structure (`Zones`, `Transport`, `Utilities`, `Power Plants`, `Buildings`) or add a new chooser group if the category no longer fits the existing layout.
7. **Check layer behavior** if the tool belongs underground (`WaterPipe`, `Subway`) or interacts with overlapping zoning/infrastructure on the surface.

### Adding a New Sim System

1. **Implement `SimSystem`** — add a new struct in `src/core/sim/systems/mod.rs` (or a new submodule) with a `tick(&mut self, map: &mut Map, sim: &mut SimState)` method.
2. **Register it** in the `SimulationEngine::new` system list in `src/core/engine.rs` at the appropriate position.
3. **Add any new fields** the system needs to `SimState` in `src/core/sim/mod.rs` (derive `Serialize`/`Deserialize` so they persist in `.tc2` saves).
4. **Optionally add an overlay** if the system produces per-tile data worth visualising: add a field to `TileOverlay` in `src/core/map/tile.rs` and handle the new overlay mode in `src/ui/game/map_view.rs` and `src/ui/theme.rs`.
5. **Add regression tests** for edge cases and forbidden states. Complex simulation bugs are much easier to prevent with focused tests than to debug after the fact.

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | Terminal UI framework — layout, widgets, rendering buffer |
| `crossterm` | 0.29 | Raw mode, mouse capture, cross-platform terminal control |
| `serde` | 1.0 | Serialisation traits |
| `serde_json` | 1.0 | Compact sim metadata inside the binary `.tc2` save container |
| `rand` | 0.8 | Map generation RNG, disaster probability rolls |

### Todos

- **Transport authenticity**: decide how far to push SC2000-style transit choice rules beyond the current simplified model. Open questions: failed-mode memory, rider split heuristics, and whether buses should get a richer simulation than depot-to-depot service on roads.
- **Statistics coverage**: extend the statistics window to show water production/consumption, trip success rate, and transport mode shares so monthly simulation output is easier to audit.
- **Testing depth**: keep expanding regression tests around layered interactions, especially binary save/load coverage, underground editing edge cases, and multi-system failures where power, water, and transport all interact.
- **Map evolution**: decide whether to add more SC2000 terrain/infrastructure behavior such as bridges, tunnels, elevation effects, seaports, and airports, or keep the current flatter city model for clarity.
- **Budget and policy depth**: decide how much of SC2000's budget/ordinance model is worth adding versus staying with the current simpler maintenance-and-tax approach.
- **Utility/transit UX**: consider whether the UI should surface more simulation diagnostics directly in inspect/tool windows, or keep the current lighter presentation and rely on overlays and tests for debugging.
- **Historical unlocks**: decide whether to expose unlock mode and year-based tool progression more visibly in the UI, including sandbox overrides inside in-game settings.

---

*TuiCity 2000 is a hobby project. Contributions, bug reports, and feature requests are welcome via GitHub Issues.*
