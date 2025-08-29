#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig};
use std::collections::{BTreeMap, BTreeSet};

use super::VariableName;

#[derive(Debug, Clone, Default)]
pub struct Solver {}

impl Solver {
    pub fn new() -> Self {
        Solver {}
    }

    fn distance_heuristic<V: VariableName>(&self, config: &Config<V>) -> f32 {
        config.max_distance_to_constraint()
    }

    fn min_f_score<'a, V: VariableName>(
        open_nodes: &mut BTreeSet<Config<'a, V>>,
        f_scores: &BTreeMap<Config<'a, V>, f32>,
    ) -> Option<Config<'a, V>> {
        use ordered_float::OrderedFloat;
        let min_config = open_nodes
            .iter()
            .min_by_key(|n| {
                OrderedFloat(
                    f_scores
                        .get(*n)
                        .copied()
                        .expect("fScore should be computed for open_nodes"),
                )
            })?
            .clone();
        open_nodes.take(&min_config)
    }
}

use super::FeasabilitySolver;

impl<V: VariableName> FeasabilitySolver<V> for Solver {
    fn restore_feasability_with_origin<'a>(
        &self,
        config: &Config<'a, V>,
        origin: Option<&FeasableConfig<'a, V>>,
    ) -> Option<FeasableConfig<'a, V>> {
        let init_g_score = 0.0f32;
        let init_f_score = init_g_score + self.distance_heuristic(config);

        let forbidden_config = origin.map(|x| x.inner().clone());

        let mut g_scores = BTreeMap::from([(config.clone(), init_g_score)]);
        let mut f_scores = BTreeMap::from([(config.clone(), init_f_score)]);

        let mut open_nodes = BTreeSet::from([config.clone()]);

        while let Some(candidate) = Self::min_f_score(&mut open_nodes, &f_scores) {
            if candidate.is_feasable()
                && !forbidden_config.as_ref().is_some_and(|x| *x == candidate)
            {
                return Some(unsafe { candidate.into_feasable_unchecked() });
            } else {
                let candidate_g_score = g_scores
                    .get(&candidate)
                    .copied()
                    .expect("Score should be computed for current node");
                for neighbour in candidate.neighbours() {
                    let tentative_g_score = candidate_g_score + 1.0f32; // neighbours are always at a distance 1 for the currently considered node

                    let neighbour_g_score =
                        g_scores.get(&neighbour).copied().unwrap_or(f32::INFINITY);
                    if tentative_g_score < neighbour_g_score {
                        let tentative_f_score =
                            tentative_g_score + self.distance_heuristic(&neighbour);
                        f_scores.insert(neighbour.clone(), tentative_f_score);
                        g_scores.insert(neighbour.clone(), tentative_g_score);
                        open_nodes.insert(neighbour);
                    }
                }
            }
        }

        None
    }
}
