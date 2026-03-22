# Changelog

All notable changes to TuiCity 2000 are documented here.

## [Unreleased]

### Screen Architecture — Shared Common Module

- **`src/ui/screens/common.rs`** — new shared module extracting ~500 lines of duplicated rendering code from 5 screen renderers (start, load_city, settings, llm_setup, theme_settings)
- **Shared primitives:** `paint_synthwave` (configurable background with sun/clouds/lit windows options), `render_footer`, `centered_panel` + `render_bordered_panel` (panel layout), `render_menu_items` (vertical selectable menu with automatic back button), color utilities (`lerp_color`, `blend_color`, `darken`), text utilities (`set_centered_string`, `truncate`)
- **Centralized back button:** `MenuConfig { back_button: true }` auto-appends a "Back" item to any menu; `render_back_button()` standalone helper for screens with custom item layouts (theme swatches, save list table)
- **Per-screen reduction:** settings.rs 364→107 lines, llm_setup.rs 440→145, theme_settings.rs 391→170, start.rs 554→290, load_city.rs 695→410
- All submenu screens (settings, llm_setup, theme_settings, load_city) now have a consistent back button at the bottom

### LLM Integration (feature = "llm")

Local LLM inference via the `candle` crate, running entirely on-device in a background thread. All LLM features are behind the `llm` cargo feature flag; the game compiles and runs identically without it.

#### City Name Generation
- **Generate Name** button on the New City screen requests an LLM-generated city name
- Polling-based: the UI shows "Generating..." while the background thread runs inference
- Without the `llm` feature the button is a no-op

#### Newspaper Stories
- On each in-game month change, the LLM is asked to write a newspaper article using current city context (population, treasury, year, demand, recent events)
- Generated stories are mixed into the news ticker alongside hardcoded stories
- Deduplicates requests per game month to avoid redundant inference

#### Advisor Panel
- New **Advisors** window (`A` shortcut, or Windows → Advisors) with 5 domain tabs: Economy, City Planning, Education, Safety, Transport
- Left/Right arrows cycle tabs; Enter requests advice for the selected domain
- Shows "Thinking..." while inference runs, then displays the LLM response
- Modal window (closes other modals when opened), closable, centered on open

#### Alert Messages
- Critical events (deficit warnings, brownouts, fires) now also submit an LLM request for a richer alert message
- The hardcoded alert fires immediately; the LLM-enhanced version appears asynchronously in the news ticker

#### Architecture
- New `src/llm/` module: `types.rs` (task/response enums, `AdvisorDomain`, `CityContext`), `context.rs` (state extraction), `prompt.rs` (prompt templates), `backend.rs` (candle model loading and generation), `mod.rs` (`LlmService` with background thread + mpsc channels)
- `LlmService` wired into `AppContext` — screens access it via `context.llm`
- Single dispatch loop in `InGameScreen::on_tick` routes `LlmResponse` by `LlmTaskTag`
- `WindowId::Advisor` added (10th managed window); `InGamePainter` trait expanded to 15 methods with `paint_advisor_window`
- Settings screen shows LLM availability status
- 6 new tests covering name generation, newspaper dedup/digest, and advisor state

### Terraforming Tools

#### In-Game Terrain Editing

- **Add Water** ($300, 1×1) — converts a grass, trees, or dirt tile to water; clears any zone on the target tile
- **Add Land** ($100, 1×1) — fills a water tile with grass
- **Plant Trees** ($20, 1×1) — places a forest tile on grass, existing trees, or dirt
- All three tools are grouped under a new **Terrain** chooser row in the toolbox (button label `[W/L/T]`)
- Placement goes through `ToolPlacer::place_tool` with a dedicated terrain branch that calls `map.set_terrain()` directly, bypassing normal occupant clearing

#### Map Generator Free Terrain Painting

- The **New City** screen now has a four-button brush selector: **None / Water / Land / Trees**
- **Mouse click** on the map preview paints the clicked tile with the active brush; clicking without a brush instead places the map cursor there and activates cursor mode
- **Mouse drag** paints every tile under the cursor as the mouse moves (button held + moved)
- **Tab** cycles the brush (None → Water → Land → Trees → None) from anywhere in the form
- **M** toggles map cursor mode; in cursor mode arrow keys move a highlighted cursor and paint the tile under it continuously; **Enter** paints without moving; **Esc** exits cursor mode
- Coordinate translation mirrors the `MapPreview` renderer's endpoint-interpolation formula exactly (`v_col * (mw-1) / (num_v_tiles_x-1)`), ensuring the painted tile is always the tile the clicked visual cell displays
- Terrain painting in the generator is free

### New Buildings and Power Plants

- **Nuclear Plant** (4×4, $15,000, unlocks 1955) — 2,000 MW; explodes on expiry like coal/gas; contributes to fire risk at gas-plant level
- **Wind Farm** (1×1, $500, unlocks 1970) — 40 MW; permanent lifespan, no EOL explosion, no pollution
- **Solar Plant** (2×2, $1,000, unlocks 1990) — 100 MW; permanent lifespan, no EOL explosion
- **Hospital** (3×3, $2,000) — raises land value within radius 4; high power (200 MW) and water (200 units) demand
- **School** (1×1, $1,000) — reduces crime within radius 8 by up to 40 points; raises land value within radius 5
- **Stadium** (4×4, $5,000) — raises land value within radius 7 by up to 35 points; high power (300 MW) and water (50 units) demand
- **Library** (1×1, $500) — reduces crime within radius 5 by up to 20 points; raises land value within radius 4
- All new buildings appear in the **Buildings** toolbox chooser; Nuclear/Wind/Solar appear in the **Power Plants** chooser

### Year-Based Tool Unlocks (Generic Availability System)

- **`ToolContext` struct** added to `src/core/tool.rs` — a generic snapshot of world state used to determine tool availability. Currently holds `year` and `unlock_mode`; designed to be extended with budget thresholds, population requirements, prerequisites, and other future conditions without changing any call sites.
- **`Tool::is_available(&ctx)`** replaces the old `is_unlocked(year, mode)` — single unified gate used by both the UI and the placement engine.
- **`Tool::unavailable_reason(&ctx)`** returns a short reason string (e.g. `"unlocks 1955"`) when a tool is locked, `None` when available. Used to annotate greyed-out items.
- **Chooser popup greys locked tools** — locked tools are rendered in the dim palette colour with the unlock year shown in parentheses (e.g. `Nuclear Plant  (unlocks 1955)`). Greyed items produce no click area and cannot be selected with the mouse.
- **Keyboard shortcuts respect locks** — direct key presses for locked tools are silently ignored; no error message is shown.
- **Sandbox mode** bypasses all year locks as before.

### Esc Deselects Active Tool

- Pressing `Esc` now follows a priority chain: close tool chooser popup → close inspect window → cancel active drag → **deselect active tool (switch to Inspect)** → open "Return to Start" confirm prompt. Previously it went straight to the quit prompt after cancelling drags.

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
