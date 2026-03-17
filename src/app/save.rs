use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::{
    map::{Map, Tile, TileOverlay},
    sim::SimState,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub sim: SimState,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<u8>,
    pub overlays: Vec<(bool, bool, u8)>,
}

#[derive(Clone)]
pub struct SaveEntry {
    pub path: PathBuf,
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub population: u64,
    pub treasury: i64,
}

pub fn saves_dir() -> PathBuf {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".tuicity2").join("saves")
}

pub fn save_city(sim: &SimState, map: &Map) -> io::Result<()> {
    let dir = saves_dir();
    fs::create_dir_all(&dir)?;

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
    let path = dir.join(&filename);

    let tiles: Vec<u8> = map.tiles.iter().map(|t| *t as u8).collect();
    let overlays: Vec<(bool, bool, u8)> = map
        .overlays
        .iter()
        .map(|o| (o.powered, o.on_fire, o.crime))
        .collect();

    let save = SaveFile {
        version: 1,
        sim: sim.clone(),
        width: map.width,
        height: map.height,
        tiles,
        overlays,
    };

    let json =
        serde_json::to_string_pretty(&save).map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(())
}

pub fn load_city(path: &Path) -> io::Result<(Map, SimState)> {
    let json = fs::read_to_string(path)?;
    let save: SaveFile =
        serde_json::from_str(&json).map_err(io::Error::other)?;

    let mut map = Map::new(save.width, save.height);
    for (i, &v) in save.tiles.iter().enumerate() {
        if i < map.tiles.len() {
            map.tiles[i] = Tile::from_u8(v);
        }
    }
    for (i, &(powered, on_fire, crime)) in save.overlays.iter().enumerate() {
        if i < map.overlays.len() {
            map.overlays[i] = TileOverlay {
                powered,
                on_fire,
                crime,
            };
        }
    }

    Ok((map, save.sim))
}

pub fn list_saves() -> Vec<SaveEntry> {
    let dir = saves_dir();
    let mut entries = Vec::new();

    let Ok(read_dir) = fs::read_dir(&dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(json) = fs::read_to_string(&path) {
            if let Ok(save) = serde_json::from_str::<SaveFile>(&json) {
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
