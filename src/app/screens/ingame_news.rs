#![allow(unused_imports)]
use std::collections::VecDeque;

use crate::{
    core::{
        map::{Map, Tile},
        sim::SimState,
    },
    ui::view::NewsTickerViewModel,
};

const SCROLL_INTERVAL_TICKS: u8 = 4;
const REFRESH_INTERVAL_TICKS: u8 = 30;
const MAX_STORIES: usize = 8;
const STORY_SEPARATOR: &str = "   ||   ";

#[derive(Debug, Clone, Default)]
pub struct CityNewsState {
    stories: Vec<String>,
    marquee_text: String,
    scroll_offset: usize,
    scroll_tick: u8,
    refresh_tick: u8,
    last_month: Option<u8>,
    last_year: Option<i32>,
    dirty: bool,
    last_alerts: CriticalAlertState,
    alerting: bool,
}

impl CityNewsState {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn tick(&mut self, sim: &SimState, map: &Map, event_messages: &VecDeque<(String, u32)>) {
        self.scroll_tick = self.scroll_tick.saturating_add(1);
        if self.scroll_tick >= SCROLL_INTERVAL_TICKS {
            self.scroll_tick = 0;
            let len = self.marquee_text.chars().count();
            if len > 0 {
                self.scroll_offset = (self.scroll_offset + 1) % len;
            }
        }

        self.refresh_tick = self.refresh_tick.saturating_add(1);
        let metrics = collect_metrics(sim, map);
        let alerts = CriticalAlertState::from_metrics(sim, &metrics);
        let month_changed = self.last_month != Some(sim.month) || self.last_year != Some(sim.year);
        let needs_refresh = self.dirty
            || self.stories.is_empty()
            || month_changed
            || alerts != self.last_alerts
            || self.refresh_tick >= REFRESH_INTERVAL_TICKS;

        if !needs_refresh {
            return;
        }

        let digest = build_news_digest(sim, &metrics, event_messages);
        self.alerting = digest.alerting;
        self.stories = digest.stories;
        self.marquee_text = build_marquee_text(&self.stories);
        let len = self.marquee_text.chars().count();
        if len > 0 {
            self.scroll_offset %= len;
        } else {
            self.scroll_offset = 0;
        }
        self.refresh_tick = 0;
        self.last_month = Some(sim.month);
        self.last_year = Some(sim.year);
        self.last_alerts = alerts;
        self.dirty = false;
    }

    pub fn view_model(&self) -> NewsTickerViewModel {
        NewsTickerViewModel {
            full_text: if self.marquee_text.is_empty() {
                "City desk warming up.   ".to_string()
            } else {
                self.marquee_text.clone()
            },
            scroll_offset: self.scroll_offset,
            is_alerting: self.alerting,
        }
    }

    #[cfg(test)]
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CriticalAlertState {
    deficit: bool,
    fires: bool,
    power_shortage: bool,
    water_shortage: bool,
}

impl CriticalAlertState {
    fn from_metrics(sim: &SimState, metrics: &CityMetrics) -> Self {
        Self {
            deficit: sim.treasury < 0 || sim.last_income < 0,
            fires: metrics.active_fires > 0,
            power_shortage: metrics.power_shortage,
            water_shortage: metrics.water_shortage,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CityMetrics {
    receivable_tiles: usize,
    unpowered_tiles: usize,
    dry_tiles: usize,
    active_fires: usize,
    road_tiles: usize,
    congested_roads: usize,
    civic_tiles: usize,
    avg_pollution: u8,
    avg_crime: u8,
    avg_fire_risk: u8,
    avg_land_value: u8,
    pop_delta: i64,
    trip_success_rate: f32,
    power_shortage: bool,
    water_shortage: bool,
}

fn collect_metrics(sim: &SimState, map: &Map) -> CityMetrics {
    let mut receivable_tiles = 0usize;
    let mut unpowered_tiles = 0usize;
    let mut dry_tiles = 0usize;
    let mut active_fires = 0usize;
    let mut road_tiles = 0usize;
    let mut congested_roads = 0usize;
    let mut civic_tiles = 0usize;
    let mut pollution_sum = 0u32;
    let mut crime_sum = 0u32;
    let mut fire_risk_sum = 0u32;
    let mut land_value_sum = 0u32;

    for (&tile, overlay) in map.tiles.iter().zip(map.overlays.iter()) {
        if overlay.on_fire {
            active_fires += 1;
        }

        if tile.is_drive_network() {
            road_tiles += 1;
            if overlay.traffic >= 128 {
                congested_roads += 1;
            }
        }

        if tile.receives_power() {
            receivable_tiles += 1;
            if !overlay.is_powered() {
                unpowered_tiles += 1;
            }
            if !overlay.has_water() {
                dry_tiles += 1;
            }
            civic_tiles += 1;
            pollution_sum += overlay.pollution as u32;
            crime_sum += overlay.crime as u32;
            fire_risk_sum += overlay.fire_risk as u32;
            land_value_sum += overlay.land_value as u32;
        }
    }

    let avg = |sum: u32| -> u8 {
        if civic_tiles == 0 {
            0
        } else {
            (sum / civic_tiles as u32).min(255) as u8
        }
    };

    let pop_delta = population_delta(sim);
    let trip_success_rate = if sim.trip_attempts == 0 {
        1.0
    } else {
        sim.trip_successes as f32 / sim.trip_attempts as f32
    };

    CityMetrics {
        receivable_tiles,
        unpowered_tiles,
        dry_tiles,
        active_fires,
        road_tiles,
        congested_roads,
        civic_tiles,
        avg_pollution: avg(pollution_sum),
        avg_crime: avg(crime_sum),
        avg_fire_risk: avg(fire_risk_sum),
        avg_land_value: avg(land_value_sum),
        pop_delta,
        trip_success_rate,
        power_shortage: sim.power_consumed_mw > sim.power_produced_mw && sim.power_consumed_mw > 0,
        water_shortage: sim.water_consumed_units > sim.water_produced_units
            && sim.water_consumed_units > 0,
    }
}

fn population_delta(sim: &SimState) -> i64 {
    let previous = if sim.population_history.len() >= 2 {
        sim.population_history[sim.population_history.len() - 2]
    } else if let Some(previous) = sim.population_history.back() {
        *previous
    } else {
        sim.population
    };
    sim.population as i64 - previous as i64
}

#[derive(Debug, Clone, Default)]
struct NewsDigest {
    stories: Vec<String>,
    alerting: bool,
}

fn build_news_digest(
    sim: &SimState,
    metrics: &CityMetrics,
    event_messages: &VecDeque<(String, u32)>,
) -> NewsDigest {
    let mut alerts = Vec::new();
    let mut events = Vec::new();
    let mut mood = Vec::new();
    let mut complaints = Vec::new();
    let mut good = Vec::new();
    let mut filler = Vec::new();

    if sim.population == 0 {
        mood.push(format!(
            "{} mood: expectant. Survey crews see empty lots and big ambitions.",
            sim.city_name
        ));
    }

    if metrics.active_fires > 0 {
        alerts.push(format!(
            "Alarm bells in {}: {} burning sites need attention right now.",
            sim.city_name, metrics.active_fires
        ));
    }
    if sim.treasury < 0 || sim.last_income < 0 {
        alerts.push(format!(
            "Budget desk: {} is bleeding cash and the books are in the red.",
            sim.city_name
        ));
    }
    if metrics.power_shortage || utility_gap(metrics.unpowered_tiles, metrics.receivable_tiles, 4) {
        alerts.push(format!(
            "Brownout bulletin: {}'s grid is straining and lights are blinking out.",
            sim.city_name
        ));
    }
    if metrics.water_shortage || utility_gap(metrics.dry_tiles, metrics.receivable_tiles, 4) {
        alerts.push(format!(
            "Waterworks warning: thirsty lots are spreading across {}.",
            sim.city_name
        ));
    }
    if sim.trip_attempts > 0 && metrics.trip_success_rate < 0.55 {
        alerts.push(
            "Commute crisis: too many trips are failing before citizens reach what they need."
                .to_string(),
        );
    }

    for (message, _) in event_messages.iter().rev().take(2) {
        events.push(format!("Bulletin: {message}"));
    }

    if mood.is_empty() {
        mood.push(overall_mood_story(sim, metrics));
    }

    if let Some(line) = dominant_complaint_story(sim, metrics) {
        complaints.push(line);
    }
    if metrics.avg_pollution >= 140 {
        complaints.push(
            "Residents complain the air tastes like factory exhaust and bad decisions.".to_string(),
        );
    }
    if metrics.avg_crime >= 140 {
        complaints.push(
            "Shopkeepers want more badges on the beat and fewer smashed windows.".to_string(),
        );
    }
    if metrics.avg_fire_risk >= 140 {
        complaints.push("Insurance clerks are sweating over the city's fire risk.".to_string());
    }
    if metrics.avg_land_value > 0 && metrics.avg_land_value <= 90 {
        complaints.push("Homeowners say property values are wilting block by block.".to_string());
    }
    if let Some(line) = weak_demand_story(sim) {
        complaints.push(line);
    }

    if sim.last_income > 0 {
        good.push(format!(
            "Revenue watch: {} is earning money instead of setting it on fire.",
            sim.city_name
        ));
    }
    if metrics.pop_delta > 0 {
        good.push(format!(
            "Population desk: {} new residents arrived since the last monthly report.",
            metrics.pop_delta
        ));
    }
    if metrics.receivable_tiles > 0 && metrics.unpowered_tiles == 0 && metrics.dry_tiles == 0 {
        good.push(
            "Utilities report: the lights are on and the taps are flowing citywide.".to_string(),
        );
    }
    if metrics.avg_pollution <= 70 && metrics.avg_crime <= 70 && metrics.civic_tiles > 0 {
        good.push("Street report: the city feels cleaner and calmer than usual.".to_string());
    }
    if transit_share_story(sim).is_some() {
        good.push(transit_share_story(sim).unwrap());
    }
    if let Some(line) = strong_demand_story(sim) {
        good.push(line);
    }

    if alerts.is_empty() && complaints.is_empty() && good.is_empty() && events.is_empty() {
        filler.push(format!(
            "Quiet shift at City Hall. {} is stable for the moment.",
            sim.city_name
        ));
    }

    let alerting = !alerts.is_empty();
    let mut stories = Vec::new();
    extend_unique(&mut stories, alerts, MAX_STORIES);
    extend_unique(&mut stories, events, MAX_STORIES);
    extend_unique(&mut stories, mood, MAX_STORIES);
    extend_unique(&mut stories, complaints, MAX_STORIES);
    extend_unique(&mut stories, good, MAX_STORIES);
    extend_unique(&mut stories, filler, MAX_STORIES);

    if stories.is_empty() {
        stories.push("City desk idle. Nobody has filed a complaint yet.".to_string());
    }

    NewsDigest { stories, alerting }
}

fn extend_unique(
    stories: &mut Vec<String>,
    incoming: impl IntoIterator<Item = String>,
    limit: usize,
) {
    for story in incoming {
        if stories.len() >= limit {
            break;
        }
        if !stories.iter().any(|existing| existing == &story) {
            stories.push(story);
        }
    }
}

fn utility_gap(problem_tiles: usize, receivable_tiles: usize, divisor: usize) -> bool {
    receivable_tiles > 0 && problem_tiles.saturating_mul(divisor) >= receivable_tiles
}

fn overall_mood_story(sim: &SimState, metrics: &CityMetrics) -> String {
    let mut score = 0i32;

    if sim.last_income > 0 {
        score += 2;
    } else if sim.last_income < 0 {
        score -= 2;
    }
    if metrics.pop_delta > 0 {
        score += 1;
    } else if metrics.pop_delta < 0 {
        score -= 1;
    }
    if metrics.active_fires > 0 {
        score -= 3;
    }
    if metrics.power_shortage || metrics.water_shortage {
        score -= 2;
    }
    if metrics.trip_success_rate >= 0.85 {
        score += 1;
    } else if metrics.trip_success_rate < 0.55 {
        score -= 1;
    }
    if metrics.avg_pollution <= 80 && metrics.civic_tiles > 0 {
        score += 1;
    } else if metrics.avg_pollution >= 140 {
        score -= 1;
    }
    if metrics.avg_crime <= 80 && metrics.civic_tiles > 0 {
        score += 1;
    } else if metrics.avg_crime >= 140 {
        score -= 1;
    }

    match score {
        4.. => format!(
            "{} mood: exuberant. Even the paper-pushers think the city is on a roll.",
            sim.city_name
        ),
        1..=3 => format!(
            "{} mood: upbeat. Citizens think City Hall might actually know what it is doing.",
            sim.city_name
        ),
        -1..=0 => format!(
            "{} mood: restless. Nobody is panicking, but patience is not infinite.",
            sim.city_name
        ),
        _ => format!(
            "{} mood: sour. Complaints are piling up faster than permits.",
            sim.city_name
        ),
    }
}

fn dominant_complaint_story(sim: &SimState, metrics: &CityMetrics) -> Option<String> {
    if metrics.active_fires > 0 {
        return Some("Fire crews say half the town smells like charcoal and overtime.".to_string());
    }
    if metrics.power_shortage || utility_gap(metrics.unpowered_tiles, metrics.receivable_tiles, 4) {
        return Some(
            "Residents complain the lights keep cutting out across whole blocks.".to_string(),
        );
    }
    if metrics.water_shortage || utility_gap(metrics.dry_tiles, metrics.receivable_tiles, 4) {
        return Some(
            "Homeowners grumble that the taps are running dry when they need them most."
                .to_string(),
        );
    }
    if sim.trip_attempts > 0 && metrics.trip_success_rate < 0.55 {
        return Some("Commuters say the transport network moves like cold syrup.".to_string());
    }
    if metrics.road_tiles > 0 && metrics.congested_roads.saturating_mul(3) >= metrics.road_tiles {
        return Some("Drivers are leaning on their horns and blaming your road plan.".to_string());
    }
    None
}

fn weak_demand_story(sim: &SimState) -> Option<String> {
    let (label, value) = weakest_demand(sim)?;
    if value > -0.15 {
        return None;
    }
    Some(match label {
        "Residential" => {
            "Housing desk: new residents are not exactly racing to move in.".to_string()
        }
        "Commercial" => "Merchants say foot traffic is not worth the rent right now.".to_string(),
        "Industrial" => "Factory owners are muttering about slow orders and idle lots.".to_string(),
        _ => return None,
    })
}

fn strong_demand_story(sim: &SimState) -> Option<String> {
    let (label, value) = strongest_demand(sim)?;
    if value < 0.45 {
        return None;
    }
    Some(match label {
        "Residential" => "Housing watch: demand for new homes is running hot.".to_string(),
        "Commercial" => "Retail watch: merchants see room to expand.".to_string(),
        "Industrial" => "Industrial watch: factories are itching to build.".to_string(),
        _ => return None,
    })
}

fn strongest_demand(sim: &SimState) -> Option<(&'static str, f32)> {
    [
        ("Residential", sim.demand_res),
        ("Commercial", sim.demand_comm),
        ("Industrial", sim.demand_ind),
    ]
    .into_iter()
    .max_by(|a, b| a.1.total_cmp(&b.1))
}

fn weakest_demand(sim: &SimState) -> Option<(&'static str, f32)> {
    [
        ("Residential", sim.demand_res),
        ("Commercial", sim.demand_comm),
        ("Industrial", sim.demand_ind),
    ]
    .into_iter()
    .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn transit_share_story(sim: &SimState) -> Option<String> {
    let transit_share = sim.bus_share + sim.rail_share + sim.subway_share;
    if sim.trip_successes == 0 || transit_share <= sim.road_share {
        return None;
    }
    Some("Transit watch: more successful trips are riding the network than driving it.".to_string())
}

fn build_marquee_text(stories: &[String]) -> String {
    if stories.is_empty() {
        return String::new();
    }
    let joined = stories.join(STORY_SEPARATOR);
    format!("{joined}{STORY_SEPARATOR}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{Map, TileOverlay};

    fn sample_city() -> (SimState, Map) {
        let mut sim = SimState::default();
        sim.city_name = "Newsville".to_string();
        sim.population = 1_250;
        sim.population_history = vec![1_000, 1_250].into();
        sim.trip_attempts = 100;
        sim.trip_successes = 90;
        sim.road_share = 40;
        sim.bus_share = 30;
        sim.rail_share = 20;
        sim.subway_share = 10;

        let mut map = Map::new(2, 2);
        map.set(0, 0, Tile::ResLow);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                power_level: 255,
                water_service: 255,
                land_value: 160,
                ..TileOverlay::default()
            },
        );
        (sim, map)
    }

    #[test]
    fn deficit_generates_finance_alert() {
        let (mut sim, map) = sample_city();
        sim.treasury = -500;
        sim.last_income = -1_000;

        let digest = build_news_digest(&sim, &collect_metrics(&sim, &map), &VecDeque::new());

        assert!(digest.alerting);
        assert!(digest
            .stories
            .iter()
            .any(|story| story.contains("books are in the red")));
    }

    #[test]
    fn fires_generate_emergency_story() {
        let (sim, mut map) = sample_city();
        let mut overlay = map.get_overlay(0, 0);
        overlay.on_fire = true;
        map.set_overlay(0, 0, overlay);

        let digest = build_news_digest(&sim, &collect_metrics(&sim, &map), &VecDeque::new());

        assert!(digest
            .stories
            .iter()
            .any(|story| story.contains("burning sites")));
    }

    #[test]
    fn poor_trip_success_generates_commuter_complaint() {
        let (mut sim, map) = sample_city();
        sim.trip_attempts = 100;
        sim.trip_successes = 25;
        sim.trip_failures = 75;

        let digest = build_news_digest(&sim, &collect_metrics(&sim, &map), &VecDeque::new());

        assert!(digest
            .stories
            .iter()
            .any(|story| story.contains("transport network")));
    }

    #[test]
    fn calm_city_still_gets_headline() {
        let (sim, map) = sample_city();

        let digest = build_news_digest(&sim, &collect_metrics(&sim, &map), &VecDeque::new());

        assert!(!digest.stories.is_empty());
    }

    #[test]
    fn ticker_state_advances_scroll_offset() {
        let (sim, map) = sample_city();
        let events = VecDeque::new();
        let mut state = CityNewsState::default();
        state.tick(&sim, &map, &events);
        let start = state.scroll_offset();

        for _ in 0..SCROLL_INTERVAL_TICKS {
            state.tick(&sim, &map, &events);
        }

        assert!(state.scroll_offset() > start);
    }
}
