# TuiCity 2000: Future Development TODOs

This document tracks planned improvements and features following the major architectural refactoring.

## ✅ Completed Milestones
- [x] **Multi-threaded Architecture**: Simulation loop moved to background thread.
- [x] **Renderer Abstraction**: Introduced `Renderer` trait to decouple logic from `ratatui`.
- [x] **UI State Machine**: Refactored monolithic frontend into modular `Screen` structs.
- [x] **Modular Simulation Engine**: Introduced `SimSystem` trait for plug-and-play game mechanics.
- [x] **Visual Demand Bars**: RCI demand is now visualized as bars in the Info Panel.
- [x] **Financial History**: Treasury history sparkline added to the Budget menu.
- [x] **Demand Trends**: Sparkline history chart for R/C/I demand in Info Panel (shown when tall enough).
- [x] **Police Coverage**: `PoliceSystem` — radius-based crime reduction; crime penalises residential/commercial growth.
- [x] **Fire Coverage**: `FireSystem` — radius-based fire risk overlay; `FireSpreadSystem` — active ignition, spreading, suppression, tile damage.
- [x] **Education/Health**: Hospitals increase nearby land value via `LandValueSystem`.
- [x] **Maintenance Costs**: Monthly deductions — roads $1/tile, power lines $1/tile, power plants $5/tile, police/fire $10/tile, parks $2/tile.
- [x] **Pollution System**: `PollutionSystem` — industrial emission + park scrubbing; pollution penalises residential growth.
- [x] **Land Value System**: `LandValueSystem` — water/park/hospital proximity bonuses; pollution penalty; boosts upgrade probability.
- [x] **Disasters Menu**: SC2000-style popup (press `D`) with checkbox toggles for Fire, Flooding, and Tornado.
- [x] **Fire Disaster**: Spontaneous ignition, tile spread, fire-station suppression, building downgrade on destruction.
- [x] **Flood Disaster**: Annual 10% chance of water spreading to 1–5 adjacent land tiles.
- [x] **Tornado Disaster**: ~2% chance/year; random destructive 3-tile-wide path across the map.

## 🎨 Visual Polish & UI
- [ ] **Dynamic Status Bar**: Implement a scrolling marquee for city news (e.g., "New power plant built", "High demand for housing").
- [ ] **Themed UI**: Use `ratatui` styling to create different "skins" for the terminal (Classic SC2K, Modern Dark, High-Contrast).

## ⚙️ Simulation Depth
- [ ] **Detailed Budget**: Break down income (taxes) vs. expenses (maintenance, services) in the Budget popup.

## 🖥️ Alternative Frontends
- [ ] **Graphical Renderer**: Create a second implementation of the `Renderer` trait using `macroquad` or `pixels`.
- [ ] **Web Backend**: Implement a renderer that pipes state to a Web-Socket for a browser-based canvas frontend.

## ⌨️ Interaction & QoL
- [ ] **Data Overlays**: Toggleable map modes (press 'V' to see Power Grid heat-map, 'C' for Crime, 'P' for Pollution).
- [ ] **Advanced Query Tool**: Clicking a building with '?' shows a detailed popup with its specific population and "happiness".
- [ ] **Auto-Save Task**: Implement a background task to save the city every 5 minutes.

## 🏗️ Refactoring & Technical Debt
- [ ] **Asset Registry**: Move hardcoded tile characters and colors into a configuration file (YAML/TOML).
- [ ] **Event-Driven Simulation**: Move from a fixed tick rate to an event-based system for certain simulation triggers.
