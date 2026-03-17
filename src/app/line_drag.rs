use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;
use crate::core::{map::Map, tool::Tool};

/// In-progress line drag (road, rail, or power line): stores the tool, start, end, and cached path.
pub struct LineDrag {
    pub tool: Tool,
    pub start_x: usize,
    pub start_y: usize,
    pub end_x: usize,
    pub end_y: usize,
    /// Cached A* (or L-shape fallback) path. Re-computed only when end changes.
    pub path: Vec<(usize, usize)>,
}

impl LineDrag {
    pub fn new(tool: Tool, x: usize, y: usize) -> Self {
        Self { tool, start_x: x, start_y: y, end_x: x, end_y: y, path: vec![(x, y)] }
    }
}

/// Manhattan distance heuristic for A*.
fn heuristic(x: usize, y: usize, ex: usize, ey: usize) -> usize {
    x.abs_diff(ex) + y.abs_diff(ey)
}

/// L-shaped fallback path (horizontal then vertical).
/// Used when A* finds no navigable route.
fn l_path(sx: usize, sy: usize, ex: usize, ey: usize) -> Vec<(usize, usize)> {
    let mut tiles = Vec::new();
    if sx <= ex { for x in sx..=ex { tiles.push((x, sy)); } }
    else        { for x in (ex..=sx).rev() { tiles.push((x, sy)); } }
    if sy < ey { for y in (sy+1)..=ey { tiles.push((ex, y)); } }
    else if ey < sy { for y in (ey..sy).rev() { tiles.push((ex, y)); } }
    tiles
}

/// Find the shortest navigable path from (sx,sy) to (ex,ey) using A* for the given tool.
/// A tile is traversable if the tool can place on it OR it already holds the tool's output tile
/// (routing through existing infrastructure of the same type is free and desirable).
/// Falls back to L-shape if no navigable path exists.
pub fn line_shortest_path(
    map: &Map,
    tool: Tool,
    sx: usize, sy: usize,
    ex: usize, ey: usize,
) -> Vec<(usize, usize)> {
    if sx == ex && sy == ey {
        return vec![(sx, sy)];
    }

    // BinaryHeap stores Reverse((f, g, x, y)) for min-heap on f = g + h
    let mut open: BinaryHeap<Reverse<(usize, usize, usize, usize)>> = BinaryHeap::new();
    let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    let mut g_score: HashMap<(usize, usize), usize> = HashMap::new();

    g_score.insert((sx, sy), 0);
    open.push(Reverse((heuristic(sx, sy, ex, ey), 0, sx, sy)));

    while let Some(Reverse((_, g, x, y))) = open.pop() {
        if (x, y) == (ex, ey) {
            let mut path = vec![(x, y)];
            let mut cur = (x, y);
            while let Some(&prev) = came_from.get(&cur) {
                path.push(prev);
                cur = prev;
            }
            path.reverse();
            return path;
        }
        // Skip stale heap entries
        if g > *g_score.get(&(x, y)).unwrap_or(&usize::MAX) {
            continue;
        }
        for (nx, ny, tile) in map.neighbors4(x, y) {
            if !tool.can_place(tile) && !tool.is_traversable(tile) { continue; }
            let ng = g + 1;
            if ng < *g_score.get(&(nx, ny)).unwrap_or(&usize::MAX) {
                g_score.insert((nx, ny), ng);
                came_from.insert((nx, ny), (x, y));
                open.push(Reverse((ng + heuristic(nx, ny, ex, ey), ng, nx, ny)));
            }
        }
    }

    // No navigable path — show L-shape (renderer will highlight blocked tiles red)
    l_path(sx, sy, ex, ey)
}
