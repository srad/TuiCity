# Changelog

All notable changes to TuiCity 2000 are documented here.

## [Unreleased]

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
