#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum TerrainTile {
    #[default]
    Grass,
    Water,
    Trees,
    Dirt,
}

/// A tile's relationship to a propagated resource (power, water).
///
/// The BFS propagation loop and post-BFS shortage scaling use this to decide:
/// - **Producer** — seeds the BFS at level 255; immune to shortage scaling.
/// - **Conductor** — relays the resource to neighbours with a per-type falloff;
///   scaled during shortage. Also receives the resource.
/// - **Consumer** — receives but does NOT relay; scaled during shortage.
/// - **None** — does not participate at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceRole {
    None,
    Producer,
    Conductor { falloff: u8 },
    Consumer,
}

impl TerrainTile {
    pub fn to_tile(self) -> Tile {
        match self {
            TerrainTile::Grass => Tile::Grass,
            TerrainTile::Water => Tile::Water,
            TerrainTile::Trees => Tile::Trees,
            TerrainTile::Dirt => Tile::Dirt,
        }
    }

    #[allow(dead_code)]
    pub fn from_tile(tile: Tile) -> Option<Self> {
        match tile {
            Tile::Grass => Some(TerrainTile::Grass),
            Tile::Water => Some(TerrainTile::Water),
            Tile::Trees => Some(TerrainTile::Trees),
            Tile::Dirt => Some(TerrainTile::Dirt),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TransportTile {
    Road,
    Rail,
    Highway,
    Onramp,
}

impl TransportTile {
    pub fn to_tile(self, has_power_line: bool) -> Tile {
        match self {
            TransportTile::Road if has_power_line => Tile::RoadPowerLine,
            TransportTile::Road => Tile::Road,
            TransportTile::Rail => Tile::Rail,
            TransportTile::Highway => Tile::Highway,
            TransportTile::Onramp => Tile::Onramp,
        }
    }

    #[allow(dead_code)]
    pub fn is_surface_road(self) -> bool {
        matches!(self, TransportTile::Road | TransportTile::Onramp)
    }

    pub fn is_drive_network(self) -> bool {
        matches!(
            self,
            TransportTile::Road | TransportTile::Highway | TransportTile::Onramp
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct UndergroundTile {
    #[serde(default)]
    pub water_pipe: bool,
    #[serde(default)]
    pub subway: bool,
}

impl UndergroundTile {
    pub fn is_empty(self) -> bool {
        !self.water_pipe && !self.subway
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ZoneKind {
    Residential,
    Commercial,
    Industrial,
}

impl ZoneKind {
    pub fn label(self) -> &'static str {
        match self {
            ZoneKind::Residential => "Residential",
            ZoneKind::Commercial => "Commercial",
            ZoneKind::Industrial => "Industrial",
        }
    }

    pub fn empty_tile(self) -> Tile {
        match self {
            ZoneKind::Residential => Tile::ZoneRes,
            ZoneKind::Commercial => Tile::ZoneComm,
            ZoneKind::Industrial => Tile::ZoneInd,
        }
    }

    pub fn from_tile(tile: Tile) -> Option<Self> {
        match tile {
            Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh => {
                Some(ZoneKind::Residential)
            }
            Tile::ZoneComm | Tile::CommLow | Tile::CommHigh => Some(ZoneKind::Commercial),
            Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy => Some(ZoneKind::Industrial),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ZoneDensity {
    Light,
    Dense,
}

impl ZoneDensity {
    pub fn label(self) -> &'static str {
        match self {
            ZoneDensity::Light => "Light",
            ZoneDensity::Dense => "Dense",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ZoneSpec {
    pub kind: ZoneKind,
    pub density: ZoneDensity,
}

impl ZoneSpec {
    pub fn empty_tile(self) -> Tile {
        self.kind.empty_tile()
    }

    #[allow(dead_code)]
    pub fn label(self) -> String {
        format!("{} {}", self.density.label(), self.kind.label())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TripMode {
    Road,
    Bus,
    Rail,
    Subway,
}

impl TripMode {
    pub fn label(self) -> &'static str {
        match self {
            TripMode::Road => "Road",
            TripMode::Bus => "Bus",
            TripMode::Rail => "Rail",
            TripMode::Subway => "Subway",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TripFailure {
    NoLocalAccess,
    NoDestination,
    NoRoute,
    TooLong,
}

impl TripFailure {
    pub fn label(self) -> &'static str {
        match self {
            TripFailure::NoLocalAccess => "No Local Access",
            TripFailure::NoDestination => "No Destination",
            TripFailure::NoRoute => "No Route",
            TripFailure::TooLong => "Too Long",
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Tile {
    #[default]
    Grass = 0,
    Water = 1,
    Trees = 2,
    Dirt = 3,
    Road = 10,
    Rail = 11,
    PowerLine = 12,
    RoadPowerLine = 13,
    Highway = 14,
    Onramp = 15,
    WaterPipe = 16,
    SubwayTunnel = 17,
    ZoneRes = 20,
    ZoneComm = 21,
    ZoneInd = 22,
    ResLow = 30,
    ResMed = 31,
    ResHigh = 32,
    CommLow = 40,
    CommHigh = 41,
    IndLight = 50,
    IndHeavy = 51,
    PowerPlantCoal = 60,
    PowerPlantGas = 61,
    Park = 62,
    Police = 63,
    Fire = 64,
    Hospital = 65,
    BusDepot = 66,
    RailDepot = 67,
    SubwayStation = 68,
    WaterPump = 69,
    WaterTower = 70,
    WaterTreatment = 71,
    Desalination = 72,
    PowerPlantNuclear = 73,
    PowerPlantWind = 74,
    School = 75,
    Stadium = 76,
    Library = 77,
    PowerPlantSolar = 78,
    Rubble = 80,
}

impl Tile {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Tile::Grass,
            1 => Tile::Water,
            2 => Tile::Trees,
            3 => Tile::Dirt,
            10 => Tile::Road,
            11 => Tile::Rail,
            12 => Tile::PowerLine,
            13 => Tile::RoadPowerLine,
            14 => Tile::Highway,
            15 => Tile::Onramp,
            16 => Tile::WaterPipe,
            17 => Tile::SubwayTunnel,
            20 => Tile::ZoneRes,
            21 => Tile::ZoneComm,
            22 => Tile::ZoneInd,
            30 => Tile::ResLow,
            31 => Tile::ResMed,
            32 => Tile::ResHigh,
            40 => Tile::CommLow,
            41 => Tile::CommHigh,
            50 => Tile::IndLight,
            51 => Tile::IndHeavy,
            60 => Tile::PowerPlantCoal,
            61 => Tile::PowerPlantGas,
            62 => Tile::Park,
            63 => Tile::Police,
            64 => Tile::Fire,
            65 => Tile::Hospital,
            66 => Tile::BusDepot,
            67 => Tile::RailDepot,
            68 => Tile::SubwayStation,
            69 => Tile::WaterPump,
            70 => Tile::WaterTower,
            71 => Tile::WaterTreatment,
            72 => Tile::Desalination,
            73 => Tile::PowerPlantNuclear,
            74 => Tile::PowerPlantWind,
            75 => Tile::School,
            76 => Tile::Stadium,
            77 => Tile::Library,
            78 => Tile::PowerPlantSolar,
            80 => Tile::Rubble,
            _ => Tile::Grass,
        }
    }

    pub fn is_zone(self) -> bool {
        matches!(self, Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd)
    }

    pub fn is_building(self) -> bool {
        matches!(
            self,
            Tile::ResLow
                | Tile::ResMed
                | Tile::ResHigh
                | Tile::CommLow
                | Tile::CommHigh
                | Tile::IndLight
                | Tile::IndHeavy
        )
    }

    pub fn is_service_building(self) -> bool {
        matches!(
            self,
            Tile::Police
                | Tile::Fire
                | Tile::Hospital
                | Tile::Park
                | Tile::BusDepot
                | Tile::RailDepot
                | Tile::SubwayStation
                | Tile::WaterPump
                | Tile::WaterTower
                | Tile::WaterTreatment
                | Tile::Desalination
                | Tile::School
                | Tile::Stadium
                | Tile::Library
        )
    }

    pub fn is_transport(self) -> bool {
        matches!(
            self,
            Tile::Road | Tile::Rail | Tile::RoadPowerLine | Tile::Highway | Tile::Onramp
        )
    }

    pub fn is_drive_network(self) -> bool {
        matches!(
            self,
            Tile::Road | Tile::RoadPowerLine | Tile::Highway | Tile::Onramp
        )
    }

    /// This tile's role in the power resource network.
    pub fn power_role(self) -> ResourceRole {
        use crate::core::sim::constants::*;
        match self {
            Tile::PowerPlantCoal
            | Tile::PowerPlantGas
            | Tile::PowerPlantNuclear
            | Tile::PowerPlantWind
            | Tile::PowerPlantSolar => ResourceRole::Producer,

            Tile::PowerLine | Tile::RoadPowerLine => {
                ResourceRole::Conductor { falloff: POWER_FALLOFF_LINE }
            }

            Tile::ResLow
            | Tile::ResMed
            | Tile::ResHigh
            | Tile::CommLow
            | Tile::CommHigh
            | Tile::IndLight
            | Tile::IndHeavy
            | Tile::Police
            | Tile::Fire
            | Tile::Hospital
            | Tile::BusDepot
            | Tile::RailDepot
            | Tile::SubwayStation
            | Tile::WaterPump
            | Tile::WaterTower
            | Tile::WaterTreatment
            | Tile::Desalination
            | Tile::School
            | Tile::Stadium
            | Tile::Library => ResourceRole::Conductor { falloff: POWER_FALLOFF_BUILDING },

            Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd => {
                ResourceRole::Conductor { falloff: POWER_FALLOFF_ZONE }
            }

            _ => ResourceRole::None,
        }
    }

    /// This tile's role in the water resource network (visible-tile side).
    ///
    /// The BFS also checks the underground pipe layer independently — if a
    /// tile has an underground pipe it conducts water regardless of the
    /// visible tile's role.
    pub fn water_role(self) -> ResourceRole {
        use crate::core::sim::constants::*;
        match self {
            Tile::WaterPump | Tile::WaterTower | Tile::WaterTreatment | Tile::Desalination => {
                ResourceRole::Producer
            }

            Tile::WaterPipe => ResourceRole::Conductor { falloff: WATER_FALLOFF_PIPE },

            Tile::ResLow
            | Tile::ResMed
            | Tile::ResHigh
            | Tile::CommLow
            | Tile::CommHigh
            | Tile::IndLight
            | Tile::IndHeavy
            | Tile::Police
            | Tile::Fire
            | Tile::Hospital
            | Tile::BusDepot
            | Tile::RailDepot
            | Tile::SubwayStation
            | Tile::School
            | Tile::Stadium
            | Tile::Library
            | Tile::ZoneRes
            | Tile::ZoneComm
            | Tile::ZoneInd => ResourceRole::Consumer,

            _ => ResourceRole::None,
        }
    }

    /// Whether this tile participates in the power network at all.
    pub fn receives_power(self) -> bool {
        self.power_role() != ResourceRole::None
    }

    /// MW consumed by this tile. Returns 0 for non-consumers.
    pub fn power_demand(self) -> u32 {
        match self {
            Tile::ResLow => 10,
            Tile::ResMed => 40,
            Tile::ResHigh => 150,
            Tile::CommLow => 30,
            Tile::CommHigh => 120,
            Tile::IndLight => 100,
            Tile::IndHeavy => 400,
            Tile::Police => 50,
            Tile::Fire => 50,
            Tile::Hospital => 200,
            Tile::BusDepot | Tile::RailDepot | Tile::SubwayStation => 25,
            Tile::WaterPump => 25,
            Tile::WaterTower => 15,
            Tile::WaterTreatment => 60,
            Tile::Desalination => 90,
            Tile::School => 50,
            Tile::Stadium => 300,
            Tile::Library => 30,
            Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd => 2,
            _ => 0,
        }
    }

    /// Water units consumed by this tile. Returns 0 for non-consumers.
    ///
    /// Zone tiles use `zone_density` for the Dense vs Light distinction;
    /// pass `None` if the tile is not a zone.
    pub fn water_demand(self, zone_density: Option<ZoneDensity>) -> u32 {
        match self {
            Tile::ResLow | Tile::CommLow | Tile::IndLight => 6,
            Tile::ResMed | Tile::CommHigh | Tile::IndHeavy => 18,
            Tile::ResHigh => 40,
            Tile::Police | Tile::Fire | Tile::Hospital => 10,
            Tile::BusDepot | Tile::RailDepot | Tile::SubwayStation => 8,
            Tile::School => 15,
            Tile::Stadium => 50,
            Tile::Library => 8,
            Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd => match zone_density {
                Some(ZoneDensity::Dense) => 4,
                Some(_) => 2,
                None => 0,
            },
            _ => 0,
        }
    }

    /// Pollution emitted by this tile per tick.
    ///
    /// `traffic` is the current traffic overlay value for the tile; it is used
    /// only by road tiles and ignored by all others.  Pass `0` for non-road tiles.
    pub fn pollution_emission(self, traffic: u8) -> u8 {
        use crate::core::sim::constants::*;
        match self {
            Tile::IndHeavy => POLLUTION_IND_HEAVY,
            Tile::IndLight => POLLUTION_IND_LIGHT,
            Tile::PowerPlantCoal => POLLUTION_COAL_PLANT,
            Tile::PowerPlantGas => POLLUTION_GAS_PLANT,
            Tile::Highway => POLLUTION_HIGHWAY,
            Tile::Road | Tile::RoadPowerLine => traffic / 4,
            _ => 0,
        }
    }

    /// If this tile actively cleans nearby pollution, returns `(radius, scrub_amount)`.
    ///
    /// During each pollution tick every tile in the square of `radius` around a cleaner
    /// has `scrub_amount` subtracted from its pollution overlay (saturating).
    /// Returns `None` for tiles that have no cleaning effect.
    pub fn pollution_cleaner(self) -> Option<(i32, u8)> {
        use crate::core::sim::constants::*;
        match self {
            Tile::Park => Some((POLLUTION_PARK_RADIUS, POLLUTION_PARK_SCRUB)),
            Tile::Trees => Some((POLLUTION_TREE_RADIUS, POLLUTION_TREE_SCRUB)),
            _ => None,
        }
    }

    pub fn road_connects(self) -> bool {
        matches!(self, Tile::Road | Tile::RoadPowerLine | Tile::Onramp)
    }

    pub fn vehicle_connects(self) -> bool {
        matches!(
            self,
            Tile::Road | Tile::RoadPowerLine | Tile::Highway | Tile::Onramp
        )
    }

    pub fn rail_connects(self) -> bool {
        self == Tile::Rail
    }

    pub fn subway_connects(self) -> bool {
        self == Tile::SubwayTunnel
    }

    pub fn water_connects(self) -> bool {
        self == Tile::WaterPipe
    }

    pub fn power_connects(self) -> bool {
        matches!(self, Tile::PowerLine | Tile::RoadPowerLine)
    }

    pub fn name(self) -> &'static str {
        match self {
            Tile::Grass => "Grass",
            Tile::Water => "Water",
            Tile::Trees => "Trees",
            Tile::Dirt => "Dirt",
            Tile::Road => "Road",
            Tile::Rail => "Rail",
            Tile::PowerLine => "Power Line",
            Tile::RoadPowerLine => "Road + Power Line",
            Tile::Highway => "Highway",
            Tile::Onramp => "Onramp",
            Tile::WaterPipe => "Water Pipe",
            Tile::SubwayTunnel => "Subway Tunnel",
            Tile::ZoneRes => "Residential Zone",
            Tile::ZoneComm => "Commercial Zone",
            Tile::ZoneInd => "Industrial Zone",
            Tile::ResLow => "Res Low-Density",
            Tile::ResMed => "Res Mid-Density",
            Tile::ResHigh => "Res High-Density",
            Tile::CommLow => "Comm Low-Density",
            Tile::CommHigh => "Comm High-Density",
            Tile::IndLight => "Light Industry",
            Tile::IndHeavy => "Heavy Industry",
            Tile::PowerPlantCoal => "Coal Power Plant",
            Tile::PowerPlantGas => "Gas Power Plant",
            Tile::Park => "Park",
            Tile::Police => "Police Dept",
            Tile::Fire => "Fire Dept",
            Tile::Hospital => "Hospital",
            Tile::BusDepot => "Bus Depot",
            Tile::RailDepot => "Rail Depot",
            Tile::SubwayStation => "Subway Station",
            Tile::WaterPump => "Water Pump",
            Tile::WaterTower => "Water Tower",
            Tile::WaterTreatment => "Water Treatment",
            Tile::Desalination => "Desalination Plant",
            Tile::PowerPlantNuclear => "Nuclear Power Plant",
            Tile::PowerPlantWind => "Wind Farm",
            Tile::School => "School",
            Tile::Stadium => "Stadium",
            Tile::Library => "Library",
            Tile::PowerPlantSolar => "Solar Power Plant",
            Tile::Rubble => "Rubble",
        }
    }

    pub fn inferred_zone_density(self) -> Option<ZoneDensity> {
        match self {
            Tile::ZoneRes
            | Tile::ZoneComm
            | Tile::ZoneInd
            | Tile::ResLow
            | Tile::CommLow
            | Tile::IndLight => Some(ZoneDensity::Light),
            Tile::ResMed | Tile::ResHigh | Tile::CommHigh | Tile::IndHeavy => {
                Some(ZoneDensity::Dense)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TileOverlay {
    /// Power level delivered to this tile. 0 = none, 255 = maximum strength.
    pub power_level: u8,
    pub on_fire: bool,
    pub crime: u8,
    #[serde(default)]
    pub zone: Option<ZoneKind>,
    /// 0 = clean air, 255 = heavily polluted (computed each tick)
    #[serde(default)]
    pub pollution: u8,
    /// 0 = lowest value, 255 = prime real estate (computed each tick)
    #[serde(default)]
    pub land_value: u8,
    /// 0 = safe, 255 = extreme risk (computed each tick, reduced by fire stations)
    #[serde(default)]
    pub fire_risk: u8,
    /// 0 = empty roads, 255 = gridlock.
    #[serde(default)]
    pub traffic: u8,
    /// 0 = dry, 255 = fully served by the water network.
    #[serde(default)]
    pub water_service: u8,
    /// Whether the lot's last monthly transport simulation found a usable trip.
    #[serde(default)]
    pub trip_success: bool,
    /// Dominant successful mode from the last monthly simulation for this lot.
    #[serde(default)]
    pub trip_mode: Option<TripMode>,
    /// Best failure reason seen across the lot's required trips when no trip succeeded.
    #[serde(default)]
    pub trip_failure: Option<TripFailure>,
    /// Last successful trip cost, normalized to the simulation's cost scale.
    #[serde(default)]
    pub trip_cost: u8,
    /// Consecutive months this building tile has been under-served
    /// (no power OR no water OR no trip). Resets when properly serviced.
    #[serde(default)]
    pub neglected_months: u8,
    /// Plant efficiency for power-plant tiles (255 = 100%, <255 = degrading near EOL).
    /// Non-plant tiles are unaffected; defaults to 255.
    #[serde(default)]
    pub plant_efficiency: u8,
}

impl TileOverlay {
    pub fn is_powered(&self) -> bool {
        self.power_level > 0
    }

    pub fn has_water(&self) -> bool {
        self.water_service > 0
    }
}
