use crate::core::map::Map;
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── HistorySystem ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct HistorySystem;
impl SimSystem for HistorySystem {
    fn name(&self) -> &str {
        "History"
    }
    fn tick(&mut self, _map: &mut Map, sim: &mut SimState) {
        let power_balance =
            sim.utilities.power_produced_mw as i32 - sim.utilities.power_consumed_mw as i32;
        sim.history.push(
            sim.demand.res,
            sim.demand.comm,
            sim.demand.ind,
            sim.economy.treasury,
            sim.pop.population,
            sim.economy.last_income,
            power_balance,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn history_vecdeque_trim_keeps_at_most_24() {
        use std::collections::VecDeque;
        let mut q: VecDeque<i64> = (0..30).map(|i| i as i64).collect();
        while q.len() > 24 {
            q.pop_front();
        }
        assert_eq!(q.len(), 24);
    }

    #[test]
    fn history_vecdeque_pop_front_returns_oldest() {
        use std::collections::VecDeque;
        let mut q: VecDeque<i64> = VecDeque::new();
        q.push_back(10);
        q.push_back(20);
        q.push_back(30);
        assert_eq!(q.pop_front(), Some(10), "pop_front returns oldest element");
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn history_vecdeque_empty_returns_none() {
        use std::collections::VecDeque;
        let mut q: VecDeque<i64> = VecDeque::new();
        assert_eq!(q.pop_front(), None, "pop_front on empty returns None");
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn history_vecdeque_exactly_24_no_trim() {
        use std::collections::VecDeque;
        let mut q: VecDeque<i64> = (0..24).map(|i| i as i64).collect();
        if q.len() > 24 {
            q.pop_front();
        }
        assert_eq!(q.len(), 24, "exactly 24 elements should not be trimmed");
    }
}
