use std::array;
use std::collections::{HashSet, VecDeque};

use crate::core::map::{Map, Tile, TransportTile, ZoneKind};

use super::WALK_DIST;

// ── Network types ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NetworkKind {
    Road,
    Rail,
    Subway,
}

#[derive(Clone, Debug, Default)]
pub(super) struct LotAccess {
    pub road_nodes: Vec<usize>,
    pub bus_nodes: Vec<usize>,
    pub rail_nodes: Vec<usize>,
    pub subway_nodes: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct ModeTargets {
    pub road_nodes: Vec<bool>,
    pub road_components: HashSet<u32>,
    pub bus_nodes: Vec<bool>,
    pub bus_components: HashSet<u32>,
    pub rail_nodes: Vec<bool>,
    pub rail_components: HashSet<u32>,
    pub subway_nodes: Vec<bool>,
    pub subway_components: HashSet<u32>,
}

impl ModeTargets {
    pub fn new(len: usize) -> Self {
        Self {
            road_nodes: vec![false; len],
            road_components: HashSet::new(),
            bus_nodes: vec![false; len],
            bus_components: HashSet::new(),
            rail_nodes: vec![false; len],
            rail_components: HashSet::new(),
            subway_nodes: vec![false; len],
            subway_components: HashSet::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct NetworkCache {
    // Connected components let the transport system reject impossible trips early before
    // paying for pathfinding. Lots on different components cannot reach each other.
    pub road_components: Vec<Option<u32>>,
    pub rail_components: Vec<Option<u32>>,
    pub subway_components: Vec<Option<u32>>,
    // Per-lot local access points into each network type.
    pub lot_access: Vec<LotAccess>,
    // Destination entry points grouped by zone kind. Empty zones are kept here so undeveloped
    // cities can still bootstrap growth instead of requiring every destination to already exist.
    pub targets_by_kind: [ModeTargets; 3],
}

impl NetworkCache {
    pub fn build(map: &Map) -> Self {
        let len = map.width * map.height;
        let road_components = label_components(map, NetworkKind::Road);
        let rail_components = label_components(map, NetworkKind::Rail);
        let subway_components = label_components(map, NetworkKind::Subway);
        let mut lot_access = vec![LotAccess::default(); len];
        let mut targets_by_kind = array::from_fn(|_| ModeTargets::new(len));

        for y in 0..map.height {
            for x in 0..map.width {
                let idx = map.idx(x, y);
                let tile = super::trip_lot_tile(map, x, y);
                if !super::is_trip_lot(tile) {
                    continue;
                }

                let access = LotAccess {
                    road_nodes: collect_local_road_nodes(map, x, y),
                    bus_nodes: collect_transfer_nodes(map, x, y, Tile::BusDepot),
                    rail_nodes: collect_transfer_nodes(map, x, y, Tile::RailDepot),
                    subway_nodes: collect_transfer_nodes(map, x, y, Tile::SubwayStation),
                };
                lot_access[idx] = access.clone();

                let Some(kind) = ZoneKind::from_tile(tile) else {
                    continue;
                };
                let targets = &mut targets_by_kind[super::zone_index(kind)];
                register_targets(
                    targets,
                    &access,
                    &road_components,
                    &rail_components,
                    &subway_components,
                );
            }
        }

        Self {
            road_components,
            rail_components,
            subway_components,
            lot_access,
            targets_by_kind,
        }
    }
}

// ── Network helpers ───────────────────────────────────────────────────────────

pub(super) fn label_components(map: &Map, network: NetworkKind) -> Vec<Option<u32>> {
    let len = map.width * map.height;
    let mut labels = vec![None; len];
    let mut next_component = 0u32;

    // A simple flood fill is enough here because connections are strictly orthogonal and
    // there are no turn penalties at the component-label stage.
    for idx in 0..len {
        if labels[idx].is_some() || !node_in_network(map, idx, network) {
            continue;
        }

        labels[idx] = Some(next_component);
        let mut queue = VecDeque::from([idx]);

        while let Some(current) = queue.pop_front() {
            let (x, y) = super::xy(map, current);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if !map.in_bounds(nx, ny) {
                    continue;
                }
                let n_idx = map.idx(nx as usize, ny as usize);
                if labels[n_idx].is_none() && node_in_network(map, n_idx, network) {
                    labels[n_idx] = Some(next_component);
                    queue.push_back(n_idx);
                }
            }
        }

        next_component += 1;
    }

    labels
}

pub(super) fn node_in_network(map: &Map, idx: usize, network: NetworkKind) -> bool {
    let (x, y) = super::xy(map, idx);
    match network {
        NetworkKind::Road => map
            .transport_at(x, y)
            .map(TransportTile::is_drive_network)
            .unwrap_or(false),
        NetworkKind::Rail => map.transport_at(x, y) == Some(TransportTile::Rail),
        NetworkKind::Subway => map.has_subway_tunnel(x, y),
    }
}

pub(super) fn collect_local_road_nodes(map: &Map, start_x: usize, start_y: usize) -> Vec<usize> {
    let mut nodes = HashSet::new();

    for_each_walkable_cell(map, start_x, start_y, WALK_DIST, |x, y| {
        // Highways intentionally do not count as walk-up local access. A lot needs a surface
        // road or a road-connected onramp before highways can extend its reach.
        if is_local_road_node(map, x, y) {
            nodes.insert(map.idx(x, y));
        }
    });

    nodes.into_iter().collect()
}

pub(super) fn collect_transfer_nodes(
    map: &Map,
    start_x: usize,
    start_y: usize,
    transfer: Tile,
) -> Vec<usize> {
    let mut nodes = HashSet::new();

    for_each_walkable_cell(map, start_x, start_y, WALK_DIST, |x, y| {
        if map.occupant_at(x, y) != Some(transfer) {
            return;
        }
        // Depots/stations are not themselves the routed network; they expose adjacent network
        // nodes that pathfinding can start from once the lot has "walked" to the transfer.
        for node in transfer_entry_nodes(map, x, y, transfer) {
            nodes.insert(node);
        }
    });

    nodes.into_iter().collect()
}

pub(super) fn transfer_entry_nodes(map: &Map, x: usize, y: usize, transfer: Tile) -> Vec<usize> {
    let mut nodes = Vec::new();
    for (nx, ny, tile) in map.neighbors4(x, y) {
        let valid = match transfer {
            Tile::BusDepot => tile.vehicle_connects(),
            Tile::RailDepot => tile == Tile::Rail,
            Tile::SubwayStation => map.has_subway_tunnel(nx, ny),
            _ => false,
        };
        if valid {
            nodes.push(map.idx(nx, ny));
        }
    }
    nodes
}

pub(super) fn for_each_walkable_cell(
    map: &Map,
    start_x: usize,
    start_y: usize,
    walk_dist: i32,
    mut f: impl FnMut(usize, usize),
) {
    let ix = start_x as i32;
    let iy = start_y as i32;

    for dy in -walk_dist..=walk_dist {
        for dx in -walk_dist..=walk_dist {
            if dx.abs() + dy.abs() > walk_dist {
                continue;
            }
            let nx = ix + dx;
            let ny = iy + dy;
            if !map.in_bounds(nx, ny) {
                continue;
            }
            f(nx as usize, ny as usize);
        }
    }
}

pub(super) fn is_local_road_node(map: &Map, x: usize, y: usize) -> bool {
    match map.transport_at(x, y) {
        Some(TransportTile::Road) => true,
        Some(TransportTile::Onramp) => map
            .neighbors4(x, y)
            .into_iter()
            .any(|(_, _, tile)| matches!(tile, Tile::Road | Tile::RoadPowerLine)),
        _ => false,
    }
}

pub(super) fn register_targets(
    targets: &mut ModeTargets,
    access: &LotAccess,
    road_components: &[Option<u32>],
    rail_components: &[Option<u32>],
    subway_components: &[Option<u32>],
) {
    // Each destination lot contributes entry nodes for every network it can be reached from.
    // Origins later search for any of these nodes when trying to satisfy a trip.
    for &node in &access.road_nodes {
        targets.road_nodes[node] = true;
        if let Some(component) = road_components[node] {
            targets.road_components.insert(component);
        }
    }
    for &node in &access.bus_nodes {
        targets.bus_nodes[node] = true;
        if let Some(component) = road_components[node] {
            targets.bus_components.insert(component);
        }
    }
    for &node in &access.rail_nodes {
        targets.rail_nodes[node] = true;
        if let Some(component) = rail_components[node] {
            targets.rail_components.insert(component);
        }
    }
    for &node in &access.subway_nodes {
        targets.subway_nodes[node] = true;
        if let Some(component) = subway_components[node] {
            targets.subway_components.insert(component);
        }
    }
}
