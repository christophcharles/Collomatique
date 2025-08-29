pub mod dijkstra;

use super::Config;

pub trait FeasabilitySolver {
    fn restore_feasability(&self, config: &Config) -> Option<Config>;
}
