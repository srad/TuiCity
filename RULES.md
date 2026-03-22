# TuiCity 2000 — Game Rules Reference

This document is the authoritative specification for all simulation mechanics in TuiCity 2000. Values are extracted directly from the source code. Constants may be adjusted in future versions.

See [CHANGELOG.md](CHANGELOG.md) for the history of changes.

---

## General

| Constant | Value | Notes |
|----------|-------|-------|
| Starting treasury | $20,000 | |
| Starting year | 1900 | |
| Starting month | 1 | January |
| History buffer size | 24 months | Rolling window for graphs |
| Month progression | 1→12, then year+1, month=1 | |

### Default State

| Field | Value |
|-------|-------|
| `demand_res` | 0.8 |
| `demand_comm` | 0.5 |
| `demand_ind` | 0.4 |
| Tax rates (all sectors) | 9% |

---

## Simulation Pipeline

Systems execute in this order every month tick:

| # | System | What it does |
|---|--------|--------------|
| 1 | `PowerSystem` | Age plants; decay efficiency in final 12 months; spread power with distance decay; explode expired plants |
| 2 | `WaterSystem` | Produce water from powered utilities; spread underground service |
| 3 | `TransportSystem` | Build network cache; simulate trips with depot capacity; accumulate traffic |
| 4 | `PollutionSystem` | Radial diffusion from industry and traffic; parks scrub nearby pollution |
| 5 | `LandValueSystem` | Score tiles: +water view, +parks, +hospital; −pollution |
| 6 | `PoliceSystem` | Set baseline crime per zone density; police stations suppress within radius 12 |
| 7 | `FireSystem` | Set baseline fire risk per zone density; fire stations suppress within radius 12 |
| 8 | `GrowthSystem` | Zone → building development; neglect tracking; brownout degradation; upgrades |
| 9 | `FireSpreadSystem` | Spontaneous ignition; spread to neighbours; suppression by fire stations; damage |
| 10 | `FloodSystem` | Trigger month 6 only; 10% annual chance |
| 11 | `TornadoSystem` | Trigger month 3 only; 2% annual chance |
| 12 | `FinanceSystem` | Maintenance costs; 1/12th tax each month; recalculate demand ratios |
| 13 | `HistorySystem` | Record metrics to rolling buffers (24 months) |

---

## Power System

### Plant Efficiency Decay

During the final 12 months of a plant's life, efficiency decays linearly:

```
efficiency = min(remaining_months / 12, 1.0)
```

- Full life (remaining ≥ 12 months): efficiency = 1.0
- 1 month remaining: efficiency ≈ 0.083
- Effective capacity = `capacity_mw × efficiency`
- All 16 footprint tiles are marked with `plant_efficiency` overlay value for the renderer
- Degraded plants display a blinking amber `!` indicator in the map view

### Plant Lifespans

| Plant | Capacity | Lifespan | Cost |
|-------|----------|----------|------|
| Coal Plant | 500 MW | 600 months (50 years) | $3,000 |
| Gas Plant | 800 MW | 720 months (60 years) | $6,000 |
| Nuclear Plant | 2000 MW | 480 months (40 years) | $15,000 | Unlocks 1955; meltdown on expiry |
| Wind Farm | 40 MW | Permanent (1×1) | $500 | Unlocks 1970; no pollution, no EOL explosion |
| Solar Plant | 100 MW | Permanent (2×2) | $1,000 | Unlocks 1990; no pollution, no EOL explosion |

### Power Consumption per Tile

| Tile | MW |
|------|----|
| ResLow | 10 |
| ResMed | 40 |
| ResHigh | 150 |
| CommLow | 30 |
| CommHigh | 120 |
| IndLight | 100 |
| IndHeavy | 400 |
| Police / Fire Dept | 50 |
| Hospital | 200 |
| BusDepot / RailDepot / SubwayStation | 25 |
| WaterPump | 25 |
| WaterTower | 15 |
| WaterTreatment | 60 |
| Desalination | 90 |
| School | 50 |
| Stadium | 300 |
| Library | 30 |
| ZoneRes / ZoneComm / ZoneInd | 2 |

### Conduction Rules

- Power propagates from plant footprint (4×4 tiles) at level 255.
- Each step away: `next_level = current_level − 2`. Propagation stops when level ≤ 1.
- **Conducts power:** plants, power lines, conductive structures (roads with power lines, developed buildings), and empty zones (ZoneRes, ZoneComm, ZoneInd).
- Roads without power lines do NOT conduct.
- Explosion on expiry: all tiles in 4×4 area become `Rubble`.

---

## Water System

### Water Production per Tile

| Facility | Production | Notes |
|----------|------------|-------|
| WaterPump | 120 | 200 if adjacent to water tile |
| WaterTower | 150 | |
| WaterTreatment | 250 | |
| Desalination | 320 | |

All require power to produce. If unpowered, production = 0.

### Water Demand per Tile

| Tile | Demand |
|------|--------|
| ResLow / CommLow / IndLight | 6 |
| ResMed / CommHigh / IndHeavy | 18 |
| ResHigh | 40 |
| Police / Fire Dept / Hospital | 10 |
| School | 15 |
| Stadium | 50 |
| Library | 8 |
| BusDepot / RailDepot / SubwayStation | 8 |
| Dense zones | 4 |
| Light zones | 2 |

### Water Conduction Rules

- Spreads underground: `next_level = current_level − 3` per tile. Stops when level ≤ 1.
- **Conducts:** pipes, water facilities, developed buildings.
- **Receives but does not relay:** empty zones.

---

## Transport System

### Network Constants

| Constant | Value |
|----------|-------|
| `WALK_DIST` | 3 tiles (Manhattan) |
| `MAX_TRIP_COST` | 48 |
| `TRANSFER_PENALTY` | 4 |
| `ROAD_TRAFFIC_FACTOR` | 4 |
| `BUS_TRAFFIC_FACTOR` | 1 |

### Trip Mode Costs

| Mode | Step cost |
|------|-----------|
| Road | 2 per tile |
| Highway | 1 per tile |
| Bus / Rail / Subway | 1 per tile |

### Trip Destination Requirements

| Origin | Destinations needed |
|--------|---------------------|
| Residential | Commercial or Industrial |
| Commercial | Residential |
| Industrial | Residential |

### Traffic Accumulation

- Successful road trips add `ROAD_TRAFFIC_FACTOR × weight` to each road tile.
- Successful bus trips add `BUS_TRAFFIC_FACTOR × weight` to each bus tile.
- Traffic decays: `traffic = saturating_sub(traffic, 16)` per month (prevents gridlock).

### Bus Depot Capacity

| Constant | Value |
|----------|-------|
| Bus Depot capacity | 100 trips per month |

When a bus depot's trip count reaches capacity, subsequent bus trips fall back to roads and apply `ROAD_TRAFFIC_FACTOR` (4) instead of `BUS_TRAFFIC_FACTOR` (1). Rail and subway depots have no capacity limit. Depot state is tracked in `sim.depots: HashMap<(x, y), DepotState>` and is reset each month. Depots are auto-registered when found on the map (for save-file compatibility), and registered on placement via the engine.

---

## Pollution System

### Pollution Sources

| Source | Strength |
|--------|----------|
| PowerPlantCoal | 250 |
| IndHeavy | 200 |
| IndLight | 120 |
| PowerPlantGas | 80 |
| Highway | 35 |
| Road | traffic / 4 |

### Diffusion

- Radius: 10 tiles (Manhattan or Euclidean? — codebase uses radial check: `dx² + dy² ≤ 100`).
- Falloff: `pollution × (1 − dist² / radius²)`.
- Clamped to 0–255.

### Park Scrubbing

- Parks remove 20 pollution from each tile within radius 3 (Euclidean).

---

## Land Value System

- Base: 80 per tile.
- Proximity bonuses use **quadratic falloff**: `bonus = max_bonus × (1 − dist² / radius²)`.
  - Water within 5 tiles: up to +40 (full bonus at source, 0 at edge).
  - Park within 4 tiles: up to +30.
  - Hospital within 4 tiles: up to +20.
- Pollution penalty: `pollution / 3`.
- Clamped to 0–255.

The quadratic curve concentrates value close to amenities (at half-radius, 75% of the bonus remains), creating stronger placement incentives than a flat step function.

---

## Police System

### Baseline Crime by Tile

| Tile | Base Crime |
|------|------------|
| ResHigh / CommHigh / IndHeavy | 90 |
| ResMed / CommLow / IndLight | 60 |
| ResLow | 40 |
| All others | 20 |

### Police Station Coverage

- Radius: 12 tiles (Euclidean).
- Max reduction: 70.
- Falloff: `reduction × (1 − dist² / 144)`.

---

## Fire System

### Baseline Fire Risk by Tile

| Tile | Base Risk |
|------|-----------|
| IndHeavy | 110 |
| IndLight | 80 |
| PowerPlantCoal | 80 |
| PowerPlantGas | 60 |
| ResHigh / CommHigh | 60 |
| ResMed / CommLow | 40 |
| ResLow | 25 |
| All others | 10 |

### Fire Station Coverage

- Radius: 12 tiles (Euclidean).
- Max reduction: 80.
- Falloff: `reduction × (1 − dist² / 144)`.

---

## Fire Spread Disaster

### Spontaneous Ignition

- Each building tile each tick: `chance = (fire_risk / 255) × 0.0002`.
- Only rolls for tiles not already on fire.

### Spread

- Each burning tile's 4-neighbors: `chance = (target_fire_risk / 255) × 0.04`.

### Damage

- Burning tiles: `chance = 1%` per tick to be destroyed.
- Downgrade path: `High → Med → Low → Zone`, then → `Grass` for non-zone tiles.

### Fire Station Suppression

- Radius: 12 tiles (Euclidean).
- `suppress_chance = 0.08 × (1 − dist² / 144)`.
- Note: at exactly dist=12, falloff=0, suppress_chance=0 (fires at the boundary are never suppressed).

---

## Flood Disaster

- **Trigger:** month 6 only.
- **Annual chance:** 10%.
- **Tiles flooded:** 1–5 randomly chosen from tiles adjacent to water (excluding roads, rails, highways, onramps).
- Flooded tiles become `Water`; fires on them are extinguished.

---

## Tornado Disaster

- **Trigger:** month 3 only.
- **Annual chance:** 2%.
- **Path:** random edge → random edge; length 12–30 tiles; swath width 3 tiles.
- Direction has 30% random drift per step, biased toward map center.
- Downgrade: `Res → ZoneRes`, `Comm → ZoneComm`, `Ind → ZoneInd`, everything else → `Grass`.

---

## Growth System

### Probabilistic Zone → Building

| Zone | Chance formula |
|------|---------------|
| ZoneRes | `(demand_res × 0.15 + lv_bonus) × pollution_penalty × crime_penalty × traffic_penalty` |
| ZoneComm | `(demand_comm × 0.08 + lv_bonus × 0.5) × crime_penalty × traffic_penalty` |
| ZoneInd | `demand_ind × 0.08 × traffic_penalty` |

Requirements: must be powered AND (`trip_success = true` OR road access within 3 tiles). The road-access fallback (bootstrap-ready) allows early growth before the transport network is complete; both paths are fully probabilistic.

### Building Upgrades

| Upgrade | Chance formula | Additional requirement |
|---------|---------------|----------------------|
| ResLow → ResMed | `(demand_res × 0.03 + lv_bonus) × penalties` | Dense zone + watered |
| ResMed → ResHigh | `(demand_res × 0.015 + lv_bonus × 0.5) × penalties` | Dense zone + watered |
| CommLow → CommHigh | `(demand_comm × 0.02 + lv_bonus × 0.5) × penalties` | Dense zone + watered |
| IndLight → IndHeavy | `demand_ind × 0.02 × traffic_penalty` | Dense zone + watered |

### Penalty Factors

| Penalty | Formula |
|---------|---------|
| Pollution | `1.0 − (pollution / 255) × 0.7` |
| Land value bonus | `(land_value / 255) × 0.1` |
| Crime | `1.0 − (crime / 255) × 0.7` |
| Traffic | `1.0 − (traffic / 255) × 0.5` |

### Degradation (Non-Functional Buildings)

| Condition | Chance | Downgrade |
|----------|--------|-----------|
| Not functional | 1% | High → Med → Low → Zone → Grass |
| ResHigh not functional | 10% | → ResMed |
| CommHigh not functional | 5% | → CommLow |
| IndHeavy not functional | 5% | → IndLight |
| Severe brownout (< 30% power) | 1% | same downgrade path |
| Fully unpowered (level = 0) | 1% | same downgrade path |

### Neglect Degradation

- A building is **underserved** if it has no power OR no water OR no successful trip.
- Each underserved month: `neglected_months = saturating_add(neglected_months, 1)`.
- At `neglected_months ≥ 6`: 1% chance per tick to downgrade one density level (same downgrade path as fire).
- When a tile is rebuilt/developed, `neglected_months = 0`.

### Brownout Thresholds

| Power level | Status |
|-------------|--------|
| < 30% of 255 (< 77) | Severe brownout |
| 0 | Fully unpowered |

---

## Finance System

### Monthly Maintenance Costs

| Tile type | Monthly cost |
|-----------|-------------|
| Road | $1 |
| Highway | $3 |
| Rail | $2 |
| PowerLine | $1 |
| WaterPipe | $1 |
| WaterPump / WaterTower / WaterTreatment / Desalination | $6 |
| Subway | $3 |
| BusDepot | $8 |
| RailDepot | $12 |
| SubwayStation | $10 |
| Police / Fire Dept | $10 |
| Park | $2 |
| Coal Plant | `(coal_tiles / 16) × $100` |
| Gas Plant | `(gas_tiles / 16) × $150` |

### Demand Calculation

```
ideal_res = 0.50
ideal_comm = 0.125
ideal_ind = 0.375

tax_modifier = (9 − tax_rate) × 0.05
growth_boost = 0.5 if population < 1000 else 0.0

demand_sector = (ideal_ratio − current_ratio + tax_modifier + growth_boost).clamp(−1.0, 1.0)
```

---

## Economy & Tax System

### Sector Capacities

| Tile | Residential pop | Commercial jobs | Industrial jobs |
|------|----------------|----------------|----------------|
| ResLow | 10 | — | — |
| ResMed | 50 | — | — |
| ResHigh | 200 | — | — |
| CommLow | — | 5 | — |
| CommHigh | — | 20 | — |
| IndLight | — | — | 10 |
| IndHeavy | — | — | 30 |

### Crime Effect on Residential

`residential_population = base_capacity × (1.0 − (crime / 255) × 0.7)`
- At crime = 255: 30% of capacity.
- At crime = 0: 100% of capacity.

### Industrial Supply Effect on Commercial

`commercial_jobs = base_capacity × (has_industrial_within_5_tiles ? 1.0 : 0.3)`

Radius: Manhattan distance ≤ 5. At least one industrial tile (IndLight or IndHeavy) in range → 100%. No industrial in range → 30%.

### Tax Formula

```
tax_per_unit = (rate / 9.0) × 5.0
annual_tax = base_capacity × tax_per_unit
monthly_collection = annual_tax / 12
```

Taxes are collected every month (1/12th), not annually. Tax rates clamped to 0–100.

---

## Tool Costs & Footprints

| Tool | Cost | Footprint |
|------|------|-----------|
| Residential Light Zone | $5 | 1×1 drag |
| Residential Dense Zone | $10 | 1×1 drag |
| Commercial Light Zone | $5 | 1×1 drag |
| Commercial Dense Zone | $10 | 1×1 drag |
| Industrial Light Zone | $5 | 1×1 drag |
| Industrial Dense Zone | $10 | 1×1 drag |
| Road | $10 | 1×1 drag |
| Highway | $40 | 1×1 drag |
| Onramp | $25 | 1×1 drag |
| Rail | $20 | 1×1 drag |
| PowerLine | $5 | 1×1 drag |
| WaterPipe | $5 | 1×1 drag |
| Subway | $50 | 1×1 drag |
| Coal Plant | $3,000 | 4×4 |
| Gas Plant | $6,000 | 4×4 |
| Water Pump | $200 | 1×1 |
| Water Tower | $350 | 2×2 |
| Water Treatment | $750 | 3×3 |
| Desalination | $1,200 | 3×3 |
| Bus Depot | $250 | 2×2 |
| Rail Depot | $500 | 2×2 |
| Subway Station | $500 | 1×1 |
| Police Station | $500 | 3×3 |
| Fire Department | $500 | 3×3 |
| Park | $80 | 2×2 |
| Bulldoze | $1 | 1×1 drag |
| Add Water | $300 | 1×1 |
| Add Land | $100 | 1×1 |
| Plant Trees | $20 | 1×1 |
| Inspect | Free | 1×1 |

### Terrain Tool Placement Rules

| Tool | Can place on |
|------|-------------|
| Add Water | Grass, Trees, Dirt — also clears any zone on the tile |
| Add Land | Water only |
| Plant Trees | Grass, Trees, Dirt |

---

## Tool Unlock Years

| Tool | Unlock year |
|------|------------|
| All except below | 1900 (always available) |
| Subway / SubwayStation | 1910 |
| Bus Depot | 1920 |
| Highway / Onramp | 1930 |

---

## Overlay Value Ranges

All overlay values are u8 (0–255).

| Field | Meaning |
|-------|---------|
| `power_level` | 0 = none, 255 = full |
| `water_service` | 0 = dry, 255 = full |
| `pollution` | 0 = clean, 255 = heavily polluted |
| `land_value` | 0 = lowest, 255 = prime real estate |
| `fire_risk` | 0 = safe, 255 = extreme risk |
| `crime` | 0 = safe, 255 = high crime |
| `traffic` | 0 = empty, 255 = gridlock |
| `trip_cost` | Normalized trip cost |
| `trip_success` | Boolean: tile had a successful trip this tick |
| `trip_mode` | Mode used for successful trip |
| `trip_failure` | Failure reason if no successful trip |
| `on_fire` | Boolean: currently burning |
| `fire_risk` | Per-tile fire risk (0–255) |
| `neglected_months` | Consecutive months tile was under-served |
| `plant_efficiency` | Plant efficiency on footprint tiles (0–255); 255 = full, <255 = decaying near EOL |
