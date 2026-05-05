use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::Var;
use collomatique_ilp::solvers::Solver;
use collomatique_ilp::{ConfigData, DefaultRepr, Variable};
use collomatique_ilp_modeler::{ConstraintSource, InternalVar, Model};
use derivative::Derivative;
use std::collections::HashMap;

pub type ProblemConstraintSource = ConstraintSource<ExtraVarName, ConstraintDesc>;
pub type ProblemInternalVar = InternalVar<Var, ExtraVarName>;
pub type IlpInnerProblem = collomatique_ilp::Problem<ProblemInternalVar, ProblemConstraintSource>;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct Problem {
    model: Model<Var, ExtraVarName, ConstraintDesc>,
    original_var_list: HashMap<Var, Variable>,
}

impl Problem {
    pub(crate) fn from_model(
        model: Model<Var, ExtraVarName, ConstraintDesc>,
        original_var_list: HashMap<Var, Variable>,
    ) -> Self {
        Problem {
            model,
            original_var_list,
        }
    }

    pub fn get_inner_problem(&self) -> &IlpInnerProblem {
        self.model.problem()
    }

    pub fn solve<'a, S>(&'a self, solver: &S) -> Option<FeasableSolution<'a>>
    where
        S: Solver<ProblemInternalVar, ProblemConstraintSource, DefaultRepr<ProblemInternalVar>>,
    {
        solver
            .solve(self.model.problem())
            .map(|feasable_config| FeasableSolution { feasable_config })
    }

    pub fn solution_from_data<'a, S>(
        &'a self,
        config_data: &ConfigData<Var>,
        solver: &S,
    ) -> Option<Solution<'a>>
    where
        S: Solver<ProblemInternalVar, ProblemConstraintSource, DefaultRepr<ProblemInternalVar>>,
    {
        if !self.check_no_missing_variables(config_data) {
            return None;
        }

        let base_values: HashMap<Var, f64> = config_data.get_values().into_iter().collect();
        let recon_problem = self.model.reconstruction_problem(&base_values).ok()?;
        let recon_sol = solver
            .solve(&recon_problem)
            .expect("There should always be a (unique!) solution to the reconstruction problem");

        let mut complete_values: HashMap<ProblemInternalVar, f64> = base_values
            .into_iter()
            .map(|(b, v)| (InternalVar::Base(b), v))
            .collect();
        complete_values.extend(recon_sol.get_values());
        let new_config_data = ConfigData::from(complete_values);

        Some(
            self.solution_from_complete_data(new_config_data)
                .expect("The configuration data should be valid!"),
        )
    }

    pub fn solution_from_complete_data<'a>(
        &'a self,
        config_data: ConfigData<ProblemInternalVar>,
    ) -> Option<Solution<'a>> {
        Some(Solution {
            config: self.model.problem().build_config(config_data).ok()?,
        })
    }

    fn check_variables_valid(&self, config_data: &ConfigData<Var>) -> bool {
        config_data
            .get_values()
            .keys()
            .all(|x| self.original_var_list.contains_key(x))
    }

    fn check_no_missing_variables(&self, config_data: &ConfigData<Var>) -> bool {
        if !self.check_variables_valid(config_data) {
            return false;
        }

        self.original_var_list
            .iter()
            .all(|(var, var_def)| match config_data.get(var.clone()) {
                Some(v) => var_def.checks_value(v),
                None => false,
            })
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""), Clone(bound = ""))]
pub struct Solution<'a> {
    config: collomatique_ilp::Config<
        'a,
        ProblemInternalVar,
        ProblemConstraintSource,
        DefaultRepr<ProblemInternalVar>,
    >,
}

impl<'a> Solution<'a> {
    pub fn get_data(&self) -> ConfigData<Var> {
        ConfigData::from(self.config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                InternalVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemInternalVar> {
        ConfigData::from(self.config.get_values())
    }

    pub fn is_feasable(&self) -> bool {
        self.config.is_feasable()
    }

    pub fn into_feasable(self) -> Option<FeasableSolution<'a>> {
        Some(FeasableSolution {
            feasable_config: self.config.into_feasable()?,
        })
    }

    pub fn blame<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<
        Item = &'b (
            collomatique_ilp::Constraint<ProblemInternalVar>,
            ProblemConstraintSource,
        ),
    > + use<'a, 'b> {
        self.config.blame()
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""), Clone(bound = ""))]
pub struct FeasableSolution<'a> {
    feasable_config: collomatique_ilp::FeasableConfig<
        'a,
        ProblemInternalVar,
        ProblemConstraintSource,
        DefaultRepr<ProblemInternalVar>,
    >,
}

impl<'a> FeasableSolution<'a> {
    pub fn into_solution(self) -> Solution<'a> {
        Solution {
            config: self.feasable_config.into_inner(),
        }
    }

    pub fn get_data(&self) -> ConfigData<Var> {
        ConfigData::from(self.feasable_config.get_values().into_iter().filter_map(
            |(var, value)| match var {
                InternalVar::Base(v) => Some((v, value)),
                _ => None,
            },
        ))
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemInternalVar> {
        ConfigData::from(self.feasable_config.get_values())
    }
}
