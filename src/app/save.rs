use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::{
    map::{Map, Tile, TileOverlay},
    sim::SimState,
};
use crate::game_info::{LEGACY_SAVE_DIR_NAMES, SAVE_DIR_NAME};

const CURRENT_SAVE_VERSION: u32 = 2;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveFile {
    #[serde(default = "current_save_version")]
    pub version: u32,
    pub sim: SimState,
    pub map: Map,
}

#[derive(serde::Deserialize)]
struct LegacySaveFile {
    #[allow(dead_code)]
    version: u32,
    sim: SimState,
    width: usize,
    height: usize,
    tiles: Vec<u8>,
    overlays: Vec<(u8, bool, u8)>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum SaveFileCompat {
    Current(SaveFile),
    Legacy(LegacySaveFile),
}

#[derive(Clone, Debug)]
pub struct SaveEntry {
    pub path: PathBuf,
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub population: u64,
    pub treasury: i64,
}

fn current_save_version() -> u32 {
    CURRENT_SAVE_VERSION
}

impl From<LegacySaveFile> for SaveFile {
    fn from(value: LegacySaveFile) -> Self {
        let mut map = Map::new(value.width, value.height);
        for (index, tile) in value.tiles.into_iter().enumerate() {
            if index < map.tiles.len() {
                map.tiles[index] = Tile::from_u8(tile);
            }
        }
        for (index, (power_level, on_fire, crime)) in value.overlays.into_iter().enumerate() {
            if index < map.overlays.len() {
                map.overlays[index] = TileOverlay {
                    power_level,
                    on_fire,
                    crime,
                    ..TileOverlay::default()
                };
            }
        }

        Self {
            version: 1,
            sim: value.sim,
            map,
        }
    }
}

pub fn app_data_dir() -> PathBuf {
    user_home_dir().join(SAVE_DIR_NAME)
}

pub fn saves_dir() -> PathBuf {
    app_data_dir().join("saves")
}

fn user_home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn save_dir_from_base(base: &Path, dir_name: &str) -> PathBuf {
    base.join(dir_name).join("saves")
}

pub fn save_city(sim: &SimState, map: &Map) -> io::Result<()> {
    save_city_in_dir(sim, map, &saves_dir()).map(|_| ())
}

pub fn save_city_in_dir(sim: &SimState, map: &Map, dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;

    let safe_name: String = sim
        .city_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("{}_{}{:02}.json", safe_name, sim.year, sim.month);
    let path = dir.join(filename);

    let save = SaveFile {
        version: CURRENT_SAVE_VERSION,
        sim: sim.clone(),
        map: map.clone(),
    };

    let json = serde_json::to_string_pretty(&save).map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

pub fn load_city(path: &Path) -> io::Result<(Map, SimState)> {
    let json = fs::read_to_string(path)?;
    let save = parse_save_file(&json)?;
    Ok((save.map, save.sim))
}

pub fn list_saves() -> Vec<SaveEntry> {
    let base = user_home_dir();
    let mut seen_paths = HashSet::new();
    let mut entries = Vec::new();

    for dir_name in std::iter::once(SAVE_DIR_NAME).chain(LEGACY_SAVE_DIR_NAMES.iter().copied()) {
        for entry in list_saves_in_dir(&save_dir_from_base(&base, dir_name)) {
            if seen_paths.insert(entry.path.clone()) {
                entries.push(entry);
            }
        }
    }

    entries.sort_by(|a, b| a.city_name.cmp(&b.city_name).then(a.year.cmp(&b.year)));
    entries
}

pub fn list_saves_in_dir(dir: &Path) -> Vec<SaveEntry> {
    let mut entries = Vec::new();

    let Ok(read_dir) = fs::read_dir(dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(json) = fs::read_to_string(&path) {
            if let Ok(save) = parse_save_file(&json) {
                entries.push(SaveEntry {
                    path,
                    city_name: save.sim.city_name.clone(),
                    year: save.sim.year,
                    month: save.sim.month,
                    population: save.sim.population,
                    treasury: save.sim.treasury,
                });
            }
        }
    }

    entries.sort_by(|a, b| a.city_name.cmp(&b.city_name).then(a.year.cmp(&b.year)));
    entries
}

fn parse_save_file(json: &str) -> io::Result<SaveFile> {
    let save = serde_json::from_str::<SaveFileCompat>(json).map_err(io::Error::other)?;
    Ok(match save {
        SaveFileCompat::Current(save) => save,
        SaveFileCompat::Legacy(save) => save.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        map::{Map, Tile, TileOverlay},
        sim::{DisasterConfig, MaintenanceBreakdown, PlantState, SimState, TaxRates},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "tuicity2000-save-tests-{}-{}-{}",
                label,
                std::process::id(),
                nonce
            ));
            fs::create_dir_all(&path).expect("temp save dir should be creatable");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_map() -> Map {
        let mut map = Map::new(4, 3);
        map.set(0, 0, Tile::Water);
        map.set(1, 0, Tile::Road);
        map.set(2, 0, Tile::ZoneRes);
        map.set(3, 0, Tile::PowerPlantCoal);
        map.set(0, 1, Tile::Police);
        map.set(1, 1, Tile::Fire);
        map.set(2, 1, Tile::Park);
        map.set(3, 1, Tile::IndHeavy);
        map.set_overlay(
            2,
            0,
            TileOverlay {
                power_level: 200,
                on_fire: false,
                crime: 17,
                pollution: 23,
                land_value: 145,
                fire_risk: 88,
            },
        );
        map.set_overlay(
            3,
            1,
            TileOverlay {
                power_level: 255,
                on_fire: true,
                crime: 92,
                pollution: 211,
                land_value: 34,
                fire_risk: 144,
            },
        );
        map
    }

    fn sample_sim() -> SimState {
        let mut sim = SimState {
            city_name: "Test City".to_string(),
            year: 1955,
            month: 8,
            treasury: 12_345,
            population: 6789,
            residential_population: 2222,
            commercial_jobs: 1111,
            industrial_jobs: 999,
            demand_res: 0.25,
            demand_comm: -0.5,
            demand_ind: 0.75,
            tax_rates: TaxRates {
                residential: 7,
                commercial: 11,
                industrial: 13,
            },
            demand_history_res: vec![0.1, 0.2, 0.3],
            demand_history_comm: vec![-0.1, -0.2],
            demand_history_ind: vec![0.4, 0.5, 0.6, 0.7],
            treasury_history: vec![100, 200, -50],
            population_history: vec![4000, 5200, 6789],
            income_history: vec![2100, 3500, 4321],
            power_balance_history: vec![20, 55, 80],
            disasters: DisasterConfig {
                fire_enabled: true,
                flood_enabled: true,
                tornado_enabled: false,
            },
            last_income: 4321,
            last_breakdown: MaintenanceBreakdown {
                roads: 10,
                power_lines: 11,
                power_plants: 12,
                police: 13,
                fire: 14,
                parks: 15,
                residential_tax: 16,
                commercial_tax: 17,
                industrial_tax: 18,
                total: 110,
                annual_tax: 999,
            },
            power_produced_mw: 500,
            power_consumed_mw: 420,
            plants: std::collections::HashMap::new(),
        };
        sim.plants.insert(
            (3, 0),
            PlantState {
                age_months: 12,
                max_life_months: 600,
                capacity_mw: 500,
            },
        );
        sim
    }

    #[test]
    fn save_and_load_roundtrip_preserves_full_state() {
        let dir = TestDir::new("roundtrip");
        let map = sample_map();
        let sim = sample_sim();

        let path = save_city_in_dir(&sim, &map, &dir.path).expect("save should succeed");
        let (loaded_map, loaded_sim) = load_city(&path).expect("load should succeed");

        assert_eq!(loaded_map, map);
        assert_eq!(loaded_sim, sim);
    }

    #[test]
    fn list_saves_reads_metadata_from_current_format() {
        let dir = TestDir::new("listing");
        let map = sample_map();
        let sim = sample_sim();

        let _ = save_city_in_dir(&sim, &map, &dir.path).expect("save should succeed");
        let saves = list_saves_in_dir(&dir.path);

        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].city_name, sim.city_name);
        assert_eq!(saves[0].year, sim.year);
        assert_eq!(saves[0].month, sim.month);
        assert_eq!(saves[0].population, sim.population);
        assert_eq!(saves[0].treasury, sim.treasury);
    }

    #[test]
    fn load_city_accepts_unknown_future_fields() {
        let dir = TestDir::new("future");
        let map = sample_map();
        let sim = sample_sim();

        let path = save_city_in_dir(&sim, &map, &dir.path).expect("save should succeed");
        let mut json = fs::read_to_string(&path).expect("saved file should be readable");
        json = json.replacen(
            "\"version\": 2,",
            "\"version\": 2,\n  \"future_top_level\": {\"enabled\": true},",
            1,
        );
        json = json.replacen(
            "\"city_name\": \"Test City\",",
            "\"city_name\": \"Test City\",\n    \"future_sim_field\": 42,",
            1,
        );
        json = json.replacen(
            "\"width\": 4,",
            "\"width\": 4,\n    \"future_map_field\": \"hello\",",
            1,
        );
        fs::write(&path, json).expect("modified save should be writable");

        let (loaded_map, loaded_sim) = load_city(&path).expect("load should tolerate new fields");

        assert_eq!(loaded_map, map);
        assert_eq!(loaded_sim, sim);
    }

    #[test]
    fn load_city_keeps_support_for_legacy_save_format() {
        let dir = TestDir::new("legacy");
        let path = dir.path.join("legacy.json");
        let sim = sample_sim();
        let legacy = serde_json::json!({
            "version": 1,
            "sim": sim,
            "width": 2,
            "height": 2,
            "tiles": [10, 60, 62, 64],
            "overlays": [
                [120, false, 3],
                [255, false, 0],
                [0, false, 2],
                [0, true, 8]
            ]
        });
        fs::write(&path, serde_json::to_string_pretty(&legacy).unwrap())
            .expect("legacy file should be writable");

        let (map, loaded_sim) = load_city(&path).expect("legacy save should load");

        assert_eq!(map.width, 2);
        assert_eq!(map.height, 2);
        assert_eq!(map.get(0, 0), Tile::Road);
        assert_eq!(map.get(1, 0), Tile::PowerPlantCoal);
        assert_eq!(map.get_overlay(0, 0).power_level, 120);
        assert_eq!(map.get_overlay(1, 1).on_fire, true);
        assert_eq!(map.get_overlay(0, 0).pollution, 0);
        assert_eq!(loaded_sim, sample_sim());
    }
}
