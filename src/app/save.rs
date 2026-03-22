use std::fs;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

use crate::core::{
    map::{
        Map, TerrainTile, Tile, TileOverlay, TransportTile, TripFailure, TripMode, UndergroundTile,
        ZoneDensity, ZoneKind,
    },
    sim::SimState,
};
use crate::game_info::SAVE_DIR_NAME;

const CURRENT_SAVE_VERSION: u32 = 7;
const BINARY_SAVE_MAGIC: [u8; 4] = *b"TC2S";
const BINARY_SAVE_EXTENSION: &str = "tc2";
#[derive(serde::Deserialize)]
struct EconomyListing {
    treasury: i64,
}
#[derive(serde::Deserialize)]
struct PopListing {
    population: u64,
}
#[derive(serde::Deserialize)]
struct SaveListingSim {
    city_name: String,
    year: i32,
    month: u8,
    pop: PopListing,
    economy: EconomyListing,
}

#[derive(Clone, Debug)]
pub struct SaveEntry {
    pub path: PathBuf,
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub population: u64,
    pub treasury: i64,
    pub modified_at: Option<SystemTime>,
}

pub enum SaveDiscoveryUpdate {
    Entry(SaveEntry),
    Finished,
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

    let path = dir.join(save_filename(&sim.city_name));
    write_binary_save_file(&path, sim, map)?;
    Ok(path)
}

fn save_filename(city_name: &str) -> String {
    // Save names are also user-visible in the load menu, so keep them readable while still
    // stripping filesystem-hostile characters.
    let safe_name: String = city_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    format!("{safe_name}.{BINARY_SAVE_EXTENSION}")
}

pub fn load_city(path: &Path) -> io::Result<(Map, SimState)> {
    load_binary_save(path)
}

pub fn delete_city(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

pub fn discover_saves(tx: Sender<SaveDiscoveryUpdate>) {
    let base = user_home_dir();
    for entry in list_saves_in_dir(&save_dir_from_base(&base, SAVE_DIR_NAME)) {
        if tx.send(SaveDiscoveryUpdate::Entry(entry)).is_err() {
            return;
        }
    }

    let _ = tx.send(SaveDiscoveryUpdate::Finished);
}

pub fn list_saves_in_dir(dir: &Path) -> Vec<SaveEntry> {
    let mut entries = Vec::new();

    let Ok(read_dir) = fs::read_dir(dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !is_supported_save_path(&path) {
            continue;
        }
        let modified_at = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        if let Ok(save) = read_save_listing(&path) {
            entries.push(SaveEntry {
                path,
                city_name: save.city_name,
                year: save.year,
                month: save.month,
                population: save.pop.population,
                treasury: save.economy.treasury,
                modified_at,
            });
        }
    }

    sort_save_entries(&mut entries);
    entries
}

pub(crate) fn sort_save_entries(entries: &mut [SaveEntry]) {
    entries.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| b.year.cmp(&a.year))
            .then_with(|| b.month.cmp(&a.month))
            .then_with(|| a.city_name.cmp(&b.city_name))
            .then_with(|| a.path.cmp(&b.path))
    });
}

fn is_supported_save_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some(BINARY_SAVE_EXTENSION)
}

fn write_binary_save_file(path: &Path, sim: &SimState, map: &Map) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let sim_json = serde_json::to_vec(sim).map_err(io::Error::other)?;

    writer.write_all(&BINARY_SAVE_MAGIC)?;
    write_u32(&mut writer, CURRENT_SAVE_VERSION)?;
    write_u32(&mut writer, sim_json.len() as u32)?;
    writer.write_all(&sim_json)?;
    write_u32(&mut writer, map.width as u32)?;
    write_u32(&mut writer, map.height as u32)?;
    writer.write_all(
        &map.terrain
            .iter()
            .copied()
            .map(encode_terrain)
            .collect::<Vec<_>>(),
    )?;
    writer.write_all(
        &map.transport
            .iter()
            .copied()
            .map(encode_transport)
            .collect::<Vec<_>>(),
    )?;
    write_packed_bools(&mut writer, &map.power_lines)?;
    writer.write_all(
        &map.underground
            .iter()
            .copied()
            .map(|cell| encode_underground(cell.unwrap_or_default()))
            .collect::<Vec<_>>(),
    )?;
    writer.write_all(
        &map.occupants
            .iter()
            .copied()
            .map(encode_tile_option)
            .collect::<Vec<_>>(),
    )?;
    writer.write_all(
        &map.zones
            .iter()
            .copied()
            .map(encode_zone_kind)
            .collect::<Vec<_>>(),
    )?;
    writer.write_all(
        &map.zone_densities
            .iter()
            .copied()
            .map(encode_zone_density)
            .collect::<Vec<_>>(),
    )?;
    for overlay in &map.overlays {
        writer.write_all(&[
            overlay.power_level,
            overlay.on_fire as u8,
            overlay.crime,
            overlay.pollution,
            overlay.land_value,
            overlay.fire_risk,
            overlay.traffic,
            overlay.water_service,
            overlay.trip_success as u8,
            overlay.trip_cost,
            encode_trip_mode(overlay.trip_mode),
            encode_trip_failure(overlay.trip_failure),
            overlay.plant_efficiency,
            overlay.neglected_months,
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn load_binary_save(path: &Path) -> io::Result<(Map, SimState)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if magic != BINARY_SAVE_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported save magic",
        ));
    }

    let _version = read_u32(&mut reader)?;
    let sim_len = read_u32(&mut reader)? as usize;
    let sim_json = read_exact_vec(&mut reader, sim_len)?;
    let sim = serde_json::from_slice::<SimState>(&sim_json).map_err(io::Error::other)?;

    let width = read_u32(&mut reader)? as usize;
    let height = read_u32(&mut reader)? as usize;
    let len = width
        .checked_mul(height)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "save dimensions overflow"))?;

    let terrain = read_exact_vec(&mut reader, len)?
        .into_iter()
        .map(decode_terrain)
        .collect::<Vec<_>>();
    let transport = read_exact_vec(&mut reader, len)?
        .into_iter()
        .map(decode_transport)
        .collect::<Vec<_>>();
    let power_lines = read_packed_bools(&mut reader, len)?;
    let underground = read_exact_vec(&mut reader, len)?
        .into_iter()
        .map(|value| {
            let cell = decode_underground(value);
            (!cell.is_empty()).then_some(cell)
        })
        .collect::<Vec<_>>();
    let occupants = read_exact_vec(&mut reader, len)?
        .into_iter()
        .map(decode_tile_option)
        .collect::<Vec<_>>();
    let zones = read_exact_vec(&mut reader, len)?
        .into_iter()
        .map(decode_zone_kind)
        .collect::<Vec<_>>();
    let zone_densities = read_exact_vec(&mut reader, len)?
        .into_iter()
        .map(decode_zone_density)
        .collect::<Vec<_>>();

    let mut overlays = Vec::with_capacity(len);
    for _ in 0..len {
        let bytes = read_exact_array::<14, _>(&mut reader)?;
        overlays.push(TileOverlay {
            power_level: bytes[0],
            on_fire: bytes[1] != 0,
            crime: bytes[2],
            zone: None,
            pollution: bytes[3],
            land_value: bytes[4],
            fire_risk: bytes[5],
            traffic: bytes[6],
            water_service: bytes[7],
            trip_success: bytes[8] != 0,
            trip_cost: bytes[9],
            trip_mode: decode_trip_mode(bytes[10]),
            trip_failure: decode_trip_failure(bytes[11]),
            plant_efficiency: bytes[12],
            neglected_months: bytes[13],
        });
    }

    let mut map = Map::new(width, height);
    map.terrain = terrain;
    map.transport = transport;
    map.power_lines = power_lines;
    map.underground = underground;
    map.occupants = occupants;
    map.zones = zones;
    map.zone_densities = zone_densities;
    map.overlays = overlays;
    map.normalize_layers();

    Ok((map, sim))
}

fn write_u32(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_exact_array::<4, _>(reader)?))
}

fn read_exact_array<const N: usize, R: Read>(reader: &mut R) -> io::Result<[u8; N]> {
    let mut buffer = [0u8; N];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn read_exact_vec(reader: &mut impl Read, len: usize) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn write_packed_bools(writer: &mut impl Write, values: &[bool]) -> io::Result<()> {
    let mut bytes = vec![0u8; values.len().div_ceil(8)];
    for (index, value) in values.iter().copied().enumerate() {
        if value {
            bytes[index / 8] |= 1 << (index % 8);
        }
    }
    writer.write_all(&bytes)
}

fn read_packed_bools(reader: &mut impl Read, len: usize) -> io::Result<Vec<bool>> {
    let bytes = read_exact_vec(reader, len.div_ceil(8))?;
    Ok((0..len)
        .map(|index| (bytes[index / 8] & (1 << (index % 8))) != 0)
        .collect())
}

fn encode_terrain(tile: TerrainTile) -> u8 {
    match tile {
        TerrainTile::Grass => 0,
        TerrainTile::Water => 1,
        TerrainTile::Trees => 2,
        TerrainTile::Dirt => 3,
    }
}

fn decode_terrain(value: u8) -> TerrainTile {
    match value {
        1 => TerrainTile::Water,
        2 => TerrainTile::Trees,
        3 => TerrainTile::Dirt,
        _ => TerrainTile::Grass,
    }
}

fn encode_transport(tile: Option<TransportTile>) -> u8 {
    match tile {
        Some(TransportTile::Road) => 0,
        Some(TransportTile::Rail) => 1,
        Some(TransportTile::Highway) => 2,
        Some(TransportTile::Onramp) => 3,
        None => u8::MAX,
    }
}

fn decode_transport(value: u8) -> Option<TransportTile> {
    match value {
        0 => Some(TransportTile::Road),
        1 => Some(TransportTile::Rail),
        2 => Some(TransportTile::Highway),
        3 => Some(TransportTile::Onramp),
        _ => None,
    }
}

fn encode_underground(tile: UndergroundTile) -> u8 {
    (tile.water_pipe as u8) | ((tile.subway as u8) << 1)
}

fn decode_underground(value: u8) -> UndergroundTile {
    UndergroundTile {
        water_pipe: (value & 0b01) != 0,
        subway: (value & 0b10) != 0,
    }
}

fn encode_tile_option(tile: Option<Tile>) -> u8 {
    tile.map(|tile| tile as u8).unwrap_or(u8::MAX)
}

fn decode_tile_option(value: u8) -> Option<Tile> {
    (value != u8::MAX).then(|| Tile::from_u8(value))
}

fn encode_zone_kind(zone: Option<ZoneKind>) -> u8 {
    match zone {
        Some(ZoneKind::Residential) => 0,
        Some(ZoneKind::Commercial) => 1,
        Some(ZoneKind::Industrial) => 2,
        None => u8::MAX,
    }
}

fn decode_zone_kind(value: u8) -> Option<ZoneKind> {
    match value {
        0 => Some(ZoneKind::Residential),
        1 => Some(ZoneKind::Commercial),
        2 => Some(ZoneKind::Industrial),
        _ => None,
    }
}

fn encode_zone_density(density: Option<ZoneDensity>) -> u8 {
    match density {
        Some(ZoneDensity::Light) => 0,
        Some(ZoneDensity::Dense) => 1,
        None => u8::MAX,
    }
}

fn decode_zone_density(value: u8) -> Option<ZoneDensity> {
    match value {
        0 => Some(ZoneDensity::Light),
        1 => Some(ZoneDensity::Dense),
        _ => None,
    }
}

fn encode_trip_mode(mode: Option<TripMode>) -> u8 {
    match mode {
        Some(TripMode::Road) => 0,
        Some(TripMode::Bus) => 1,
        Some(TripMode::Rail) => 2,
        Some(TripMode::Subway) => 3,
        None => u8::MAX,
    }
}

fn decode_trip_mode(value: u8) -> Option<TripMode> {
    match value {
        0 => Some(TripMode::Road),
        1 => Some(TripMode::Bus),
        2 => Some(TripMode::Rail),
        3 => Some(TripMode::Subway),
        _ => None,
    }
}

fn encode_trip_failure(failure: Option<TripFailure>) -> u8 {
    match failure {
        Some(TripFailure::NoLocalAccess) => 0,
        Some(TripFailure::NoDestination) => 1,
        Some(TripFailure::NoRoute) => 2,
        Some(TripFailure::TooLong) => 3,
        None => u8::MAX,
    }
}

fn decode_trip_failure(value: u8) -> Option<TripFailure> {
    match value {
        0 => Some(TripFailure::NoLocalAccess),
        1 => Some(TripFailure::NoDestination),
        2 => Some(TripFailure::NoRoute),
        3 => Some(TripFailure::TooLong),
        _ => None,
    }
}

fn read_save_listing(path: &Path) -> io::Result<SaveListingSim> {
    read_binary_save_listing(path)
}

fn read_binary_save_listing(path: &Path) -> io::Result<SaveListingSim> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if magic != BINARY_SAVE_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported save magic",
        ));
    }

    let _version = read_u32(&mut reader)?;
    let sim_len = read_u32(&mut reader)? as usize;
    let sim_json = read_exact_vec(&mut reader, sim_len)?;
    serde_json::from_slice::<SaveListingSim>(&sim_json).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        map::{Map, Tile, TileOverlay},
        sim::{
            DemandState, DisasterConfig, EconomyState, HistoryState, MaintenanceBreakdown,
            PlantState, PopulationState, RngState, SimState, TaxRates, TripStats, UtilityState,
        },
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
                zone: Some(crate::core::map::ZoneKind::Residential),
                pollution: 23,
                land_value: 145,
                fire_risk: 88,
                traffic: 0,
                water_service: 0,
                ..TileOverlay::default()
            },
        );
        map.set_overlay(
            3,
            1,
            TileOverlay {
                power_level: 255,
                on_fire: true,
                crime: 92,
                zone: Some(crate::core::map::ZoneKind::Industrial),
                pollution: 211,
                land_value: 34,
                fire_risk: 144,
                traffic: 0,
                water_service: 0,
                ..TileOverlay::default()
            },
        );
        map
    }

    fn sample_sim() -> SimState {
        let mut sim = SimState {
            city_name: "Test City".to_string(),
            year: 1955,
            month: 8,
            economy: EconomyState {
                treasury: 12_345,
                tax_rates: TaxRates {
                    residential: 7,
                    commercial: 11,
                    industrial: 13,
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
                unlock_mode: crate::core::sim::UnlockMode::Historical,
            },
            pop: PopulationState {
                population: 6789,
                residential_population: 2222,
                commercial_jobs: 1111,
                industrial_jobs: 999,
            },
            demand: DemandState {
                res: 0.25,
                comm: -0.5,
                ind: 0.75,
            },
            history: HistoryState {
                demand_res: vec![0.1, 0.2, 0.3].into(),
                demand_comm: vec![-0.1, -0.2].into(),
                demand_ind: vec![0.4, 0.5, 0.6, 0.7].into(),
                treasury: vec![100, 200, -50].into(),
                population: vec![4000, 5200, 6789].into(),
                income: vec![2100, 3500, 4321].into(),
                power_balance: vec![20, 55, 80].into(),
            },
            utilities: UtilityState {
                power_produced_mw: 500,
                power_consumed_mw: 420,
                water_produced_units: 300,
                water_consumed_units: 180,
            },
            trips: TripStats {
                attempts: 70,
                successes: 54,
                failures: 16,
                road_share: 20,
                bus_share: 12,
                rail_share: 14,
                subway_share: 8,
            },
            rng: RngState {
                transport: 0x0123_4567_89AB_CDEF,
                disaster: 0xBEEF_DEAD_BEEF_CAFE,
                growth: 0xFACEFEED_FACEFEED,
            },
            disasters: DisasterConfig {
                fire_enabled: true,
                flood_enabled: true,
                tornado_enabled: false,
            },
            plants: std::collections::HashMap::new(),
            depots: std::collections::HashMap::new(),
        };
        sim.plants.insert(
            (3, 0),
            PlantState {
                age_months: 12,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
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
        assert_eq!(
            path.extension().and_then(|ext| ext.to_str()),
            Some(BINARY_SAVE_EXTENSION)
        );
        let (loaded_map, loaded_sim) = load_city(&path).expect("load should succeed");

        assert_eq!(loaded_map, map);
        assert_eq!(loaded_sim, sim);
    }

    #[test]
    fn save_and_load_roundtrip_preserves_layered_map_state() {
        let dir = TestDir::new("layered-roundtrip");
        let mut map = Map::new(6, 4);
        map.set(0, 0, Tile::Water);
        map.set_zone(1, 1, Some(crate::core::map::ZoneKind::Residential));
        map.set_transport(1, 1, Some(crate::core::map::TransportTile::Road));
        map.set_zone(2, 1, Some(crate::core::map::ZoneKind::Commercial));
        map.set_power_line(2, 1, true);
        map.set_zone(3, 1, Some(crate::core::map::ZoneKind::Industrial));
        map.set_occupant(3, 1, Some(Tile::IndLight));
        map.set_power_line(3, 1, true);
        map.set_transport(4, 1, Some(crate::core::map::TransportTile::Rail));
        map.set_overlay(
            2,
            1,
            TileOverlay {
                power_level: 180,
                on_fire: false,
                crime: 9,
                zone: Some(crate::core::map::ZoneKind::Commercial),
                pollution: 12,
                land_value: 140,
                fire_risk: 7,
                traffic: 0,
                water_service: 0,
                ..TileOverlay::default()
            },
        );

        let sim = sample_sim();
        let path = save_city_in_dir(&sim, &map, &dir.path).expect("save should succeed");
        let (loaded_map, loaded_sim) = load_city(&path).expect("load should succeed");

        assert_eq!(loaded_sim, sim);
        assert_eq!(loaded_map, map);
        assert_eq!(
            loaded_map.zone_kind(1, 1),
            Some(crate::core::map::ZoneKind::Residential)
        );
        assert_eq!(loaded_map.get(1, 1), Tile::Road);
        assert!(loaded_map.has_power_line(2, 1));
        assert_eq!(loaded_map.get(2, 1), Tile::PowerLine);
        assert_eq!(loaded_map.occupant_at(3, 1), Some(Tile::IndLight));
        assert_eq!(loaded_map.get(3, 1), Tile::IndLight);
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
        assert_eq!(saves[0].population, sim.pop.population);
        assert_eq!(saves[0].treasury, sim.economy.treasury);
        assert!(saves[0].modified_at.is_some());
    }

    #[test]
    fn saving_same_city_reuses_the_same_file() {
        let dir = TestDir::new("overwrite");
        let map = sample_map();
        let mut initial = sample_sim();
        let first_path =
            save_city_in_dir(&initial, &map, &dir.path).expect("first save should succeed");

        initial.year += 1;
        initial.month = 1;
        initial.pop.population += 100;
        let second_path =
            save_city_in_dir(&initial, &map, &dir.path).expect("second save should succeed");

        assert_eq!(first_path, second_path);
        assert_eq!(
            first_path.file_name().and_then(|name| name.to_str()),
            Some("Test_City.tc2")
        );

        let saves = list_saves_in_dir(&dir.path);
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].year, initial.year);
        assert_eq!(saves[0].month, initial.month);
        assert_eq!(saves[0].population, initial.pop.population);
    }

    #[test]
    fn list_saves_orders_most_recently_saved_first() {
        let dir = TestDir::new("recent-first");
        let map = sample_map();

        let mut older = sample_sim();
        older.city_name = "Older City".to_string();
        let _ = save_city_in_dir(&older, &map, &dir.path).expect("older save should succeed");

        std::thread::sleep(Duration::from_millis(1100));

        let mut newer = sample_sim();
        newer.city_name = "Newer City".to_string();
        newer.year = 1962;
        newer.month = 4;
        let _ = save_city_in_dir(&newer, &map, &dir.path).expect("newer save should succeed");

        let saves = list_saves_in_dir(&dir.path);

        assert_eq!(saves.len(), 2);
        assert_eq!(saves[0].city_name, newer.city_name);
        assert_eq!(saves[1].city_name, older.city_name);
    }
}
