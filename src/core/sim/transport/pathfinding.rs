use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::core::map::{Map, TransportTile, TripFailure, TripMode};

use super::TRANSFER_PENALTY;

// ── Path types ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct RouteSuccess {
    pub path: Vec<usize>,
    pub cost: usize,
}

// ── Dijkstra ──────────────────────────────────────────────────────────────────

pub(super) fn search_path(
    map: &Map,
    starts: &[usize],
    targets: &[bool],
    mode: TripMode,
    max_cost: usize,
) -> Result<RouteSuccess, TripFailure> {
    if starts.is_empty() {
        return Err(TripFailure::NoLocalAccess);
    }

    // Dijkstra over a tiny grid is easier to reason about than a more specialized router,
    // and the weighted costs let highways and transit feel different without extra systems.
    let len = map.width * map.height;
    let mut open: BinaryHeap<Reverse<(usize, usize)>> = BinaryHeap::new();
    let mut came_from: Vec<Option<usize>> = vec![None; len];
    let mut costs = vec![usize::MAX; len];
    let mut hit_limit = false;
    let initial_cost = match mode {
        TripMode::Road => 0,
        TripMode::Bus | TripMode::Rail | TripMode::Subway => TRANSFER_PENALTY,
    };

    for &start in starts {
        if costs[start] > initial_cost {
            costs[start] = initial_cost;
            open.push(Reverse((initial_cost, start)));
        }
    }

    while let Some(Reverse((cost, idx))) = open.pop() {
        if cost > costs[idx] {
            continue;
        }
        if cost > max_cost {
            hit_limit = true;
            continue;
        }
        if targets[idx] {
            return Ok(RouteSuccess {
                path: reconstruct_path(&came_from, idx),
                cost,
            });
        }

        for next in neighbors_for_mode(map, idx, mode) {
            let Some(step_cost) = step_cost(map, next, mode) else {
                continue;
            };
            let next_cost = cost + step_cost;
            if next_cost > max_cost {
                hit_limit = true;
                continue;
            }
            if next_cost < costs[next] {
                costs[next] = next_cost;
                came_from[next] = Some(idx);
                open.push(Reverse((next_cost, next)));
            }
        }
    }

    if hit_limit {
        Err(TripFailure::TooLong)
    } else {
        Err(TripFailure::NoRoute)
    }
}

fn reconstruct_path(came_from: &[Option<usize>], goal: usize) -> Vec<usize> {
    let mut path = vec![goal];
    let mut current = goal;
    while let Some(prev) = came_from[current] {
        path.push(prev);
        current = prev;
    }
    path.reverse();
    path
}

fn neighbors_for_mode(map: &Map, idx: usize, mode: TripMode) -> Vec<usize> {
    let (x, y) = super::xy(map, idx);
    let mut neighbors = Vec::with_capacity(4);
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if !map.in_bounds(nx, ny) {
            continue;
        }
        let n_idx = map.idx(nx as usize, ny as usize);
        if step_cost(map, n_idx, mode).is_some() {
            neighbors.push(n_idx);
        }
    }
    neighbors
}

pub(super) fn step_cost(map: &Map, idx: usize, mode: TripMode) -> Option<usize> {
    let (x, y) = super::xy(map, idx);
    match mode {
        TripMode::Road => match map.transport_at(x, y)? {
            TransportTile::Road => Some(2),
            TransportTile::Onramp => Some(2),
            TransportTile::Highway => Some(1),
            TransportTile::Rail => None,
        },
        TripMode::Bus => map
            .transport_at(x, y)
            .filter(|transport| transport.is_drive_network())
            .map(|_| 1),
        TripMode::Rail => map
            .transport_at(x, y)
            .filter(|transport| *transport == TransportTile::Rail)
            .map(|_| 1),
        TripMode::Subway => map.has_subway_tunnel(x, y).then_some(1),
    }
}
