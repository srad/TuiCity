pub mod growth;

use crate::core::map::{Map, Tile};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub treasury: i64,
    pub population: u64,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            city_name: "Unnamed City".to_string(),
            year: 1900,
            month: 1,
            treasury: 20_000,
            population: 0,
            demand_res: 0.8,
            demand_comm: 0.5,
            demand_ind: 0.4,
        }
    }
}

impl SimState {
    pub fn advance_month(&mut self, map: &mut Map) {
        growth::tick_growth(map, self);

        self.month += 1;
        if self.month > 12 {
            self.month = 1;
            self.year += 1;
            // Annual tax: $5 per resident
            self.treasury += (self.population as i64) * 5;
        }

        // Recalculate demand based on zone/building ratios
        let res = count_tiles(map, |t| {
            matches!(
                t,
                Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh
            )
        }) as f32;
        let comm = count_tiles(map, |t| {
            matches!(
                t,
                Tile::ZoneComm | Tile::CommLow | Tile::CommHigh
            )
        }) as f32;
        let ind = count_tiles(map, |t| {
            matches!(
                t,
                Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy
            )
        }) as f32;

        let total = (res + comm + ind).max(1.0);
        self.demand_res = (1.0 - (res / total).min(0.9)).max(0.1);
        self.demand_comm = (1.0 - (comm / total).min(0.9)).max(0.1);
        self.demand_ind = (1.0 - (ind / total).min(0.9)).max(0.1);
    }

    pub fn month_name(&self) -> &'static str {
        match self.month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???",
        }
    }
}

fn count_tiles(map: &Map, pred: impl Fn(Tile) -> bool) -> usize {
    map.tiles.iter().filter(|&&t| pred(t)).count()
}
