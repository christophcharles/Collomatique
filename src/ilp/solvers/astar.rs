#[cfg(test)]
mod tests;

use crate::ilp::ndtools::ConfigRepr;
use crate::ilp::{Config, FeasableConfig, Problem};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct Solver<'a> {
    problem: &'a Problem,
}

impl<'a> Solver<'a> {
    pub fn new(problem: &'a Problem) -> Self {
        Solver { problem }
    }

    fn distance_heuristic(&self, _config: &ConfigRepr) -> f32 {
        0.
    }

    fn min_f_score(
        open_nodes: &mut BTreeSet<ConfigRepr<'a>>,
        f_scores: &BTreeMap<ConfigRepr<'a>, f32>,
    ) -> Option<ConfigRepr<'a>> {
        use ordered_float::OrderedFloat;

        let (min_config, _) = f_scores.iter().min_by_key(|(_, s)| OrderedFloat(**s))?;
        open_nodes.take(min_config)
    }
}

use super::FeasabilitySolver;

impl<'a> FeasabilitySolver<'a> for Solver<'a> {
    fn restore_feasability_exclude(
        &self,
        config: &Config<'a>,
        exclude_list: &BTreeSet<&FeasableConfig>,
    ) -> Option<FeasableConfig<'a>> {
        let config_repr = self.problem.mat_repr.config(config);
        let init_g_score = 0.0f32;
        let init_f_score = init_g_score + self.distance_heuristic(&config_repr);

        let exclude_configs: BTreeSet<ConfigRepr<'_>> =
            exclude_list.iter().map(|x| x.inner().repr()).collect();

        let mut g_scores = BTreeMap::from([(config_repr.clone(), init_g_score)]);
        let mut f_scores = BTreeMap::from([(config_repr.clone(), init_f_score)]);

        let mut open_nodes = BTreeSet::from([config_repr]);

        while let Some(candidate) = Self::min_f_score(&mut open_nodes, &f_scores) {
            if candidate.is_feasable() && !exclude_configs.contains(&candidate) {
                return Some(unsafe { Config::from(candidate).into_feasable_unchecked() });
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
