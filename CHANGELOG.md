# Changelog

All notable changes to TuiCity 2000 are documented here.

## [Unreleased]

### Multi-Tile Building Art

- **Per-position character art for footprint buildings** — multi-tile buildings (Police, Fire, WaterTreatment, Desalination, PowerPlantCoal, PowerPlantGas, Park, WaterTower, BusDepot, RailDepot) now render with unique characters at each tile position instead of repeating the same glyph. Buildings display box-drawing frames (`┌─┐│└┘`) with labels and interior detail, making them visually distinct structures on the map.
- **Render-time position inference** — tile position within a building is inferred by scanning same-tile neighbours (no map data or save format changes). A new `building_offset()` function in `map_view.rs` determines each tile's (dx, dy) offset within its footprint.
- **Central art table** — 10 building art definitions (`POLICE_ART`, `FIRE_ART`, `COAL_PLANT_ART`, etc.) stored as `(left_char, right_char)` arrays in `theme.rs`, indexed row-major by position.
- **Footprint preview shows building art** — the placement preview now displays the actual building art shape instead of a uniform character, so players see the final structure before committing.

### Event Loop Architecture — Responsiveness

- **Blocking engine thread** — replaced the `try_recv()` + `sleep(10ms)` polling loop with blocking `recv()`. The engine command thread now wakes instantly when a command arrives (zero latency, down from up to 10ms). After processing the first command, remaining queued commands are drained while still holding the write lock to avoid lock churn during bursts. Applied to both terminal and pixel frontends.
- **Event drain loop** — the terminal event loop now drains all pending crossterm events before rendering the next frame, preventing a one-event-per-frame backlog during fast mouse movement or rapid keypresses.
- **Always-tick guarantee** — `on_tick()` now fires every frame regardless of whether input events arrived. Previously it only ran when `event::poll` timed out, causing animations (fire flicker, power pulse, news ticker) and the simulation clock to freeze during active mouse interaction.

### UI Architecture — InGamePainter Trait

- **Dual-frontend rendering trait** — extracted `InGamePainter` trait in `src/ui/painter.rs` with 14 methods covering every in-game UI element (map, minimap, toolbar, menu bar, menu popup, status bar, budget, inspect, statistics, tool chooser, help, about, legend, news ticker). Both the terminal (`TerminalPainter`) and pixel (`PixelPainter`) frontends implement the same trait, sharing a single `orchestrate_ingame()` orchestrator that drives paint order and click-area bookkeeping.
- **Menu popup z-order fix** — the status bar was rendering after the menu popup, overwriting its border. Reordered the orchestrator so `paint_status_bar` runs before `paint_menu_popup`, ensuring the popup always draws on top.
- **Menu toggle on header click** — clicking an already-open menu header (e.g. "File" while the File menu is visible) now closes the menu instead of re-opening it.

### Legacy Cleanup

- **Unified duplicate `StatusBarAreas`** — removed the local copy in `game::statusbar.rs`; both frontends now use the single definition in `painter.rs`
- **Removed `Tool::ALL`** — unused const deleted from `core/tool.rs`
- **Removed `AppContext.running`** — the field was never read; all screens use `app.running` directly
- **Blanket `#![allow(dead_code)]` removed** from `core/engine.rs`, `core/map/mod.rs`, and `core/map/tile.rs`; replaced with targeted `#[allow(dead_code)]` on 10 data-model methods that may be needed later
- **Blanket `#[allow(unused_imports)]` removed** from `core/engine.rs`, `app/screens/ingame.rs`, `app/screens/ingame_news.rs`, and `ui/game/map_view.rs`; dead imports deleted, test-only imports moved to `#[cfg(test)]` modules
- **Test-only methods gated with `#[cfg(test)]`** — `is_confirm_prompt_open`, `menu_row`, `menu_action_for`, and `tile_at_minimap_click` no longer compile into release builds

### UI & Rendering Fixes

- **Power line on zone tile now visible** — a power line placed over an undeveloped zone previously rendered with its own dark background, making it invisible against the zone colour. The map renderer now detects the underlying zone via `surface_lot_tile` and draws the power line glyph with the zone's background colour instead. Added `power_line_over_zone_uses_zone_background` regression test.
- **Start menu cleaned up** — removed the Music Toggle item from the start screen. "Quit Game" is now the last entry (index 3). Music is toggled in-game via **File → Toggle Music** only.

### Simulation Fixes

- **Empty zones relay power (SC2000 parity)** — `Tile::is_conductive_structure()` now includes empty zone tiles (`ZoneRes`, `ZoneComm`, `ZoneInd`). Power lines threaded through undeveloped zones correctly chain electricity to the next tile, matching SimCity 2000 behaviour.

### Code Quality

- **`TileCtx` test field cleanup** — removed the now-obsolete `covered_by_power_line` and `has_road_access` fields from the growth test helpers after those fields were eliminated from the struct. Fixed three locations in `growth.rs` test code.

### Simulation Engine Refactoring

**Goal:** Improve maintainability and readability without changing any game mechanics. All 259 tests pass.

- **Centralized constants** — extracted all magic numbers from 5+ files into `src/core/sim/constants.rs` (~30 named `pub const` values covering service radii, strengths, propagation falloff, transport limits, economy ratios, fire/crime baselines, plant lifecycle, and neglect thresholds)
- **`for_each_in_radius` helper** — eliminated 4 copies of the radial-iteration-with-falloff pattern; extracted into `src/core/sim/util.rs` with `(nx, ny, idx, falloff)` closure API
- **`SimState::push_history`** — atomic method that pushes to all 7 VecDeque ring buffers and trims to `HISTORY_LEN` in one call; `debug_assert` enforces all deques stay in sync
- **`tick_growth` sub-functions** — extracted `evaluate_res`, `evaluate_comm`, `evaluate_ind` (each returning `Option<Tile>`) and a `TileCtx` struct to eliminate a large multi-responsibility match block
- **`systems/` file split** — broke the 2079-line `systems/mod.rs` monolith into 10 focused files (`power.rs`, `water.rs`, `pollution.rs`, `land_value.rs`, `police.rs`, `fire.rs`, `growth_system.rs`, `finance.rs`, `history.rs`, `disasters.rs`); `mod.rs` is now re-exports only
- **`FireSpreadSystem` sub-phases** — refactored into 4 named private functions: `ignite_spontaneous`, `spread_fires`, `apply_fire_damage`, `suppress_with_stations`
- **System ordering assertion** — `#[cfg(debug_assertions)]` check in `SimulationEngine::new` ensures the 13-system pipeline is registered in the semantically correct order
- **Engine constants** — `engine.rs` plant placement now uses named constants instead of inline literals
- **22 new tests** (259 total): 6 in `constants.rs`, 5 in `util.rs`, 4 in `sim/mod.rs` (`push_history`), 7 in `growth.rs` (`evaluate_*` functions)

### Phase 5: Power Plant Efficiency Decay
- Plants now degrade in efficiency over their final 12 months of life
- Efficiency formula: `remaining_months / 12` (linear decay from 1.0 → ~0.083)
- Effective power capacity scales with efficiency: `capacity_mw * efficiency`
- All 16 footprint tiles marked with `plant_efficiency` overlay value for renderer
- Blinking amber `'!'` indicator on degraded plants in map view
- Inspect popup shows capacity (MW), efficiency %, and remaining months
- Binary save format bumped to v6 (backward compatible with v5)
- 6 new tests covering full-lifecycle, EOL boundary, one-month remaining, overlay propagation, brownout scaling, and normal plant preservation
- `capacity_mw` field added to `PlantInfo` and displayed in inspect popup

## [0.1.1] — 2025-03 — Performance Optimizations

### Phase 4a — Finance single-pass tile counting
- Replaced ~16 separate map iterations with two single-pass structures (`TileCounts`, `UndergroundCounts`)
- `TileCounts::count` iterates all surface tiles once; `UndergroundCounts::count` iterates underground tiles once
- `RoadPowerLine` correctly counted as both road and power line tile

### Phase 4b — VecDeque history ring buffer
- Migrated 7 history vectors to `VecDeque` for O(1) ring-buffer trimming
- `HistorySystem::tick` now uses `push_back()` and `pop_front()` instead of `Vec::remove(0)`
- serde-compatible (VecDeque serializes identically to Vec via default container impl)
- 12 new tests added across all Phase 4 sub-phases (231 total)

### Phase 4c — Batch overlay clearing
- Added `Map::reset_service_overlays()` for single-pass clearing of power, water, and trip service fields
- Consolidated 3 separate overlay-reset loops into 1 (at the start of PowerSystem::tick)
- `pollution`, `crime`, `fire_risk`, `neglected_months`, `on_fire` intentionally preserved (handled by their own systems)

## [0.1.0] — 2025-03 — Simulation Overhaul

### Phase 1 — Foundation
- Removed dead `TrafficSystem` wrapper (was never registered in the engine pipeline)
- Replaced `thread_rng()` with seeded `StdRng` in disaster and growth systems for reproducibility
- Fixed fire suppression radius inconsistency: `FireSpreadSystem` radius 8 → 12 (matching `FireSystem`)

### Phase 2 — Responsiveness
- Added `neglected_months` tracking: buildings degrade after 6 consecutive months without power, water, or functional trips
- Brownout degradation: severe brownout (< 30% power) and fully unpowered tiles degrade at 1% per tick
- Crime reduces residential capacity: up to 70% penalty at max crime (matching growth formula)
- `tick_growth` rewritten cleanly, removing accumulated duplicate code from editing sessions

### Phase 3 — Economy & Transit
- Tax collection monthly (1/12th of annual each month, SC2000-correct) with annualized `last_income`
- Commercial requires industrial supply chain: 30% capacity without nearby industry, 100% with (Manhattan radius 5)
- Bus Depot capacity: 100 trips/month; excess trips fall back to roads with `ROAD_TRAFFIC_FACTOR` (4)
- Rail and subway depots: no capacity limit
- Depots auto-registered on map scan and on placement via engine; removed on bulldoze
