use collo_ml::SqliteDatabaseConnection;
use collo_ml::eval::Origin;
use collo_ml::problem::ReifiedVar;
use collomatique_binding_colloscopes::vars::Var;
use collomatique_ilp::ConfigData;
use collomatique_ilp::DefaultRepr;
use collomatique_ilp::solvers::Solver;
use collomatique_ilp_modeler::{ConstraintSource, InternalVar};

pub type ProblemConstraintSource = ConstraintSource<
    ReifiedVar<SqliteDatabaseConnection>,
    Option<Origin<SqliteDatabaseConnection>>,
>;
pub type ProblemInternalVar = InternalVar<Var, ReifiedVar<SqliteDatabaseConnection>>;
pub type IlpInnerProblem = collomatique_ilp::Problem<ProblemInternalVar, ProblemConstraintSource>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    inner: collo_ml::problem::Problem<SqliteDatabaseConnection, Var>,
}

impl Problem {
    pub(crate) fn from_inner(
        inner: collo_ml::problem::Problem<SqliteDatabaseConnection, Var>,
    ) -> Self {
        Problem { inner }
    }

    pub fn get_inner_problem(&self) -> &IlpInnerProblem {
        self.inner.get_inner_problem()
    }

    pub fn solve<'a, S>(&'a self, solver: &S) -> Option<FeasableSolution<'a>>
    where
        S: Solver<ProblemInternalVar, ProblemConstraintSource, DefaultRepr<ProblemInternalVar>>,
    {
        self.inner
            .solve(solver)
            .map(|inner| FeasableSolution { inner })
    }

    pub fn solution_from_data<'a, S>(
        &'a self,
        config_data: &ConfigData<Var>,
        solver: &S,
    ) -> Option<Solution<'a>>
    where
        S: Solver<ProblemInternalVar, ProblemConstraintSource, DefaultRepr<ProblemInternalVar>>,
    {
        self.inner
            .solution_from_data(config_data, solver)
            .map(|inner| Solution { inner })
    }

    pub fn solution_from_complete_data<'a>(
        &'a self,
        config_data: ConfigData<ProblemInternalVar>,
    ) -> Option<Solution<'a>> {
        self.inner
            .solution_from_complete_data(config_data)
            .map(|inner| Solution { inner })
    }
}

#[derive(Debug, Clone)]
pub struct Solution<'a> {
    inner: collo_ml::problem::Solution<'a, SqliteDatabaseConnection, Var>,
}

impl<'a> Solution<'a> {
    pub fn get_data(&self) -> ConfigData<Var> {
        self.inner.get_data()
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemInternalVar> {
        self.inner.get_complete_data()
    }

    pub fn is_feasable(&self) -> bool {
        self.inner.is_feasable()
    }

    pub fn into_feasable(self) -> Option<FeasableSolution<'a>> {
        self.inner
            .into_feasable()
            .map(|inner| FeasableSolution { inner })
    }

    pub fn blame<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<
        Item = &'b (
            collomatique_ilp::Constraint<ProblemInternalVar>,
            ProblemConstraintSource,
        ),
    > + use<'a, 'b> {
        self.inner.blame()
    }
}

#[derive(Debug, Clone)]
pub struct FeasableSolution<'a> {
    inner: collo_ml::problem::FeasableSolution<'a, SqliteDatabaseConnection, Var>,
}

impl<'a> FeasableSolution<'a> {
    pub fn into_solution(self) -> Solution<'a> {
        Solution {
            inner: self.inner.into_solution(),
        }
    }

    pub fn get_data(&self) -> ConfigData<Var> {
        self.inner.get_data()
    }

    pub fn get_complete_data(&self) -> ConfigData<ProblemInternalVar> {
        self.inner.get_complete_data()
    }
}
