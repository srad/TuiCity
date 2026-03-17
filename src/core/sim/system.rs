use crate::core::map::Map;
use crate::core::sim::SimState;
use std::fmt::Debug;

pub trait SimSystem: Debug + Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn tick(&mut self, map: &mut Map, sim: &mut SimState);
}
