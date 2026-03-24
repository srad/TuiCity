/// Central repository for all simulation balance constants.
/// Changing a value here automatically updates every system that uses it.

// ── Service radii ─────────────────────────────────────────────────────────────

/// Radius (in tiles) within which a police station reduces crime.
pub const POLICE_RADIUS: i32 = 12;

/// Radius (in tiles) within which a fire station reduces fire risk.
pub const FIRE_STATION_RADIUS: i32 = 12;

/// Radius (in tiles) within which a fire station can suppress active fires.
pub const FIRE_SPREAD_SUPPRESS_RADIUS: i32 = 12;

/// Radius (in tiles) for pollution diffusion from industrial sources.
pub const POLLUTION_RADIUS: i32 = 10;

// ── Service strengths ─────────────────────────────────────────────────────────

/// Maximum crime reduction (at distance 0) applied by a police station.
pub const POLICE_CRIME_REDUCTION: f32 = 70.0;

/// Maximum fire risk reduction (at distance 0) applied by a fire station.
pub const FIRE_RISK_REDUCTION: f32 = 80.0;

/// Base probability per tick that a fire station suppresses an adjacent fire (at distance 0).
pub const FIRE_SUPPRESS_CHANCE_BASE: f32 = 0.08;

// ── Power / water propagation ─────────────────────────────────────────────────

/// Power level drop per tile for dedicated power lines (best conductor).
pub const POWER_FALLOFF_LINE: u8 = 1;

/// Power level drop per tile for buildings and service structures.
pub const POWER_FALLOFF_BUILDING: u8 = 3;

/// Power level drop per tile for undeveloped zones (poor conductor).
pub const POWER_FALLOFF_ZONE: u8 = 8;

/// Water service level drop per tile for underground pipes.
pub const WATER_FALLOFF_PIPE: u8 = 2;

/// Water service level drop per tile for water facility-to-facility relay.
pub const WATER_FALLOFF_FACILITY: u8 = 1;

// ── Transport ─────────────────────────────────────────────────────────────────

/// Maximum number of tiles a lot may walk to reach a road, depot, or station.
pub const WALK_DIST: i32 = 3;

/// Hard cap on Dijkstra path cost; trips exceeding this are marked as failures.
pub const MAX_TRIP_COST: usize = 48;

/// Extra cost added when switching between transport modes (e.g. walk → bus).
pub const TRANSFER_PENALTY: usize = 4;

/// Each road tile traversed adds this many units to the map-level traffic counter.
pub const ROAD_TRAFFIC_FACTOR: u16 = 4;

/// Monthly trip budget per bus depot before it falls back to road-only routing.
pub const BUS_DEPOT_CAPACITY: u32 = 100;

// ── Finance / economy ─────────────────────────────────────────────────────────

/// Tax rate (%) at which the demand modifier is zero; deviations shift demand linearly.
pub const TAX_BASELINE_RATE: f32 = 9.0;

/// Target fraction of all zone/building tiles that should be residential.
pub const IDEAL_RES_RATIO: f32 = 0.50;

/// Target fraction of all zone/building tiles that should be commercial.
pub const IDEAL_COMM_RATIO: f32 = 0.125;

/// Target fraction of all zone/building tiles that should be industrial.
pub const IDEAL_IND_RATIO: f32 = 0.375;

/// Population below which a flat demand boost is applied to all sectors.
pub const GROWTH_BOOST_THRESHOLD: u64 = 1000;

/// Flat demand boost applied when population is below `GROWTH_BOOST_THRESHOLD`.
pub const GROWTH_BOOST_AMOUNT: f32 = 0.5;

// ── Fire risk baselines ───────────────────────────────────────────────────────

pub const FIRE_RISK_IND_HEAVY: u8 = 110;
pub const FIRE_RISK_IND_LIGHT_OR_COAL: u8 = 80;
pub const FIRE_RISK_RES_HIGH_OR_COMM_HIGH_OR_GAS: u8 = 60;
pub const FIRE_RISK_RES_MED_OR_COMM_LOW: u8 = 40;
pub const FIRE_RISK_RES_LOW: u8 = 25;
pub const FIRE_RISK_DEFAULT: u8 = 10;

// ── Crime baselines ───────────────────────────────────────────────────────────

pub const CRIME_BASE_HIGH_DENSITY: u8 = 90;
pub const CRIME_BASE_MED_DENSITY: u8 = 60;
pub const CRIME_BASE_RES_LOW: u8 = 40;
pub const CRIME_BASE_DEFAULT: u8 = 20;

// ── Power plant lifecycle ─────────────────────────────────────────────────────

/// Number of months before end-of-life over which plant efficiency linearly decays to 0.
pub const EOL_DECAY_MONTHS: u32 = 12;

pub const COAL_PLANT_LIFE_MONTHS: u32 = 50 * 12;
pub const GAS_PLANT_LIFE_MONTHS: u32 = 60 * 12;
pub const COAL_PLANT_CAPACITY_MW: u32 = 500;
pub const GAS_PLANT_CAPACITY_MW: u32 = 800;

pub const NUCLEAR_PLANT_LIFE_MONTHS: u32 = 40 * 12; // 40-year license
pub const NUCLEAR_PLANT_CAPACITY_MW: u32 = 2000;

/// Wind farms are effectively permanent (≈833 years) — they never trigger meltdown.
pub const WIND_FARM_LIFE_MONTHS: u32 = u32::MAX;
pub const WIND_FARM_CAPACITY_MW: u32 = 40; // per turbine tile

/// Solar power plants are effectively permanent and clean.
pub const SOLAR_PLANT_LIFE_MONTHS: u32 = u32::MAX;
pub const SOLAR_PLANT_CAPACITY_MW: u32 = 100; // per tile (2×2)

// ── Neglect / brownout thresholds ─────────────────────────────────────────────

/// Consecutive months a building must be underserved before it starts degrading.
pub const NEGLECT_THRESHOLD_MONTHS: u8 = 6;

/// Fraction of full power demand below which a brownout degradation tick may fire.
pub const BROWNOUT_THRESHOLD: f32 = 0.30;

// ── Disaster probabilities ────────────────────────────────────────────────────

/// Max probability per tick that a building at full fire_risk spontaneously ignites.
pub const FIRE_IGNITE_CHANCE_MAX: f32 = 0.0002;

/// Max probability per tick that a fire spreads to a fully-at-risk adjacent building.
pub const FIRE_SPREAD_CHANCE_MAX: f32 = 0.04;

/// Probability per tick that a burning building is destroyed (downgraded one tier).
pub const FIRE_DAMAGE_CHANCE: f32 = 0.01;

/// Probability that a flood event fires in the trigger month.
pub const FLOOD_TRIGGER_CHANCE: f32 = 0.10;

/// Probability that a tornado event fires in the trigger month.
pub const TORNADO_TRIGGER_CHANCE: f32 = 0.02;

/// Probability per step that a tornado changes its horizontal direction.
pub const TORNADO_DRIFT_CHANCE: f32 = 0.3;

// ── Land value ────────────────────────────────────────────────────────────────

/// Baseline land value applied to every tile before bonuses/penalties.
pub const LV_BASELINE: u16 = 80;

/// Water proximity bonus: radius in tiles and maximum bonus at source.
pub const LV_WATER_RADIUS: i32 = 5;
pub const LV_WATER_BONUS: f32 = 40.0;

/// Park proximity bonus: radius in tiles and maximum bonus at source.
pub const LV_PARK_RADIUS: i32 = 4;
pub const LV_PARK_BONUS: f32 = 30.0;

/// Hospital proximity bonus: radius in tiles and maximum bonus at source.
pub const LV_HOSPITAL_RADIUS: i32 = 4;
pub const LV_HOSPITAL_BONUS: f32 = 20.0;

/// School proximity bonus and crime reduction.
pub const LV_SCHOOL_RADIUS: i32 = 5;
pub const LV_SCHOOL_BONUS: f32 = 25.0;
pub const SCHOOL_CRIME_RADIUS: i32 = 8;
pub const SCHOOL_CRIME_REDUCTION: f32 = 40.0;

/// Stadium proximity bonus (large civic attractor).
pub const LV_STADIUM_RADIUS: i32 = 7;
pub const LV_STADIUM_BONUS: f32 = 35.0;

/// Library proximity bonus and mild crime reduction.
pub const LV_LIBRARY_RADIUS: i32 = 4;
pub const LV_LIBRARY_BONUS: f32 = 20.0;
pub const LIBRARY_CRIME_RADIUS: i32 = 5;
pub const LIBRARY_CRIME_REDUCTION: f32 = 20.0;

/// Pollution divisor: each point of pollution reduces land value by 1/LV_POLLUTION_DIVISOR.
pub const LV_POLLUTION_DIVISOR: u16 = 3;

// ── History ring buffers ──────────────────────────────────────────────────────

/// Number of months of history retained in `SimState` VecDeque ring buffers.
pub const HISTORY_LEN: usize = 24;

#[cfg(test)]
mod tests {
    use super::*;

    /// Spot-check that service radii match the documented SC2000-equivalent values.
    #[test]
    fn service_radii_match_documented_values() {
        assert_eq!(POLICE_RADIUS, 12);
        assert_eq!(FIRE_STATION_RADIUS, 12);
        assert_eq!(FIRE_SPREAD_SUPPRESS_RADIUS, 12);
        assert_eq!(POLLUTION_RADIUS, 10);
    }

    /// Ensure the ideal zone ratios sum to exactly 1.0 (floating-point tolerance).
    #[test]
    fn ideal_zone_ratios_sum_to_one() {
        let sum = IDEAL_RES_RATIO + IDEAL_COMM_RATIO + IDEAL_IND_RATIO;
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "ideal ratios must sum to 1.0, got {sum}"
        );
    }

    /// Power and water falloff values must be positive (otherwise BFS never terminates).
    #[test]
    fn propagation_falloffs_are_positive() {
        assert!(POWER_FALLOFF_LINE > 0);
        assert!(POWER_FALLOFF_BUILDING > 0);
        assert!(POWER_FALLOFF_ZONE > 0);
        assert!(WATER_FALLOFF_PIPE > 0);
        assert!(WATER_FALLOFF_FACILITY > 0);
    }

    /// History ring buffer must hold at least one entry.
    #[test]
    fn history_len_is_at_least_one() {
        assert!(HISTORY_LEN >= 1);
    }

    /// Plant capacities and lifetimes must be non-zero.
    #[test]
    fn plant_constants_are_non_zero() {
        assert!(COAL_PLANT_CAPACITY_MW > 0);
        assert!(GAS_PLANT_CAPACITY_MW > 0);
        assert!(COAL_PLANT_LIFE_MONTHS > 0);
        assert!(GAS_PLANT_LIFE_MONTHS > 0);
        assert!(EOL_DECAY_MONTHS > 0);
    }

    /// BUS_DEPOT_CAPACITY must be positive (zero would mean all trips fall back to roads).
    #[test]
    fn bus_depot_capacity_is_positive() {
        assert!(BUS_DEPOT_CAPACITY > 0);
    }

    /// Disaster probabilities must be in (0, 1) — zero means it never fires, ≥1 is always.
    #[test]
    fn disaster_probabilities_are_in_unit_range() {
        assert!(FIRE_IGNITE_CHANCE_MAX > 0.0 && FIRE_IGNITE_CHANCE_MAX < 1.0);
        assert!(FIRE_SPREAD_CHANCE_MAX > 0.0 && FIRE_SPREAD_CHANCE_MAX < 1.0);
        assert!(FIRE_DAMAGE_CHANCE > 0.0 && FIRE_DAMAGE_CHANCE < 1.0);
        assert!(FLOOD_TRIGGER_CHANCE > 0.0 && FLOOD_TRIGGER_CHANCE < 1.0);
        assert!(TORNADO_TRIGGER_CHANCE > 0.0 && TORNADO_TRIGGER_CHANCE < 1.0);
        assert!(TORNADO_DRIFT_CHANCE > 0.0 && TORNADO_DRIFT_CHANCE < 1.0);
    }

    /// Land value radii must be positive; bonuses must be positive; divisor must be non-zero.
    #[test]
    fn land_value_constants_are_valid() {
        assert!(LV_BASELINE > 0);
        assert!(LV_WATER_RADIUS > 0);
        assert!(LV_WATER_BONUS > 0.0);
        assert!(LV_PARK_RADIUS > 0);
        assert!(LV_PARK_BONUS > 0.0);
        assert!(LV_HOSPITAL_RADIUS > 0);
        assert!(LV_HOSPITAL_BONUS > 0.0);
        assert!(LV_POLLUTION_DIVISOR > 0);
        // Bonuses must fit in u8 (land value is written back as u8)
        assert!(LV_WATER_BONUS < 256.0);
        assert!(LV_PARK_BONUS < 256.0);
        assert!(LV_HOSPITAL_BONUS < 256.0);
    }
}
