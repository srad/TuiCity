pub mod growth;

use crate::core::map::{Map, Tile};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimState {
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub treasury: i64,
    pub population: u64,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
    pub tax_rate: u8,
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
            tax_rate: 9, // 9% is the neutral tax rate in SC2000
        }
    }
}

impl SimState {
    pub fn advance_month(&mut self, map: &mut Map) {
        map.update_power_grid();
        growth::tick_growth(map, self);

        self.month += 1;
        if self.month > 12 {
            self.month = 1;
            self.year += 1;
            // Annual tax collection based on tax_rate
            // Assuming average income is roughly proportional to population
            // SC2000 uses more complex formulas, but this gives a lever to the player.
            let tax_per_capita = (self.tax_rate as f32 / 9.0) * 5.0; 
            self.treasury += (self.population as f32 * tax_per_capita) as i64;
        }

        // Recalculate demand based on SC2000 ideal 4:1:3 ratio (50% R, 12.5% C, 37.5% I)
        let res = count_tiles(map, |t| {
            matches!(
                t,
                Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh
            )
        }) as f32;
        let comm = count_tiles(map, |t| {
            matches!(t, Tile::ZoneComm | Tile::CommLow | Tile::CommHigh)
        }) as f32;
        let ind = count_tiles(map, |t| {
            matches!(t, Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy)
        }) as f32;

        let total = (res + comm + ind).max(1.0);
        let current_res_ratio = res / total;
        let current_comm_ratio = comm / total;
        let current_ind_ratio = ind / total;

        let ideal_res = 0.50;
        let ideal_comm = 0.125;
        let ideal_ind = 0.375;

        // Base demand is the difference between ideal and current ratio
        let base_res_demand = ideal_res - current_res_ratio;
        let base_comm_demand = ideal_comm - current_comm_ratio;
        let base_ind_demand = ideal_ind - current_ind_ratio;

        // Tax penalty/bonus: neutral is 9%.
        // Higher taxes reduce demand globally.
        let tax_modifier = (9.0 - self.tax_rate as f32) * 0.05;
        
        // Add general growth factor if population is small, to bootstrap the city
        let growth_boost = if self.population < 1000 { 0.5 } else { 0.0 };

        self.demand_res = (base_res_demand + tax_modifier + growth_boost).clamp(-1.0, 1.0);
        self.demand_comm = (base_comm_demand + tax_modifier + growth_boost).clamp(-1.0, 1.0);
        self.demand_ind = (base_ind_demand + tax_modifier + growth_boost).clamp(-1.0, 1.0);
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
