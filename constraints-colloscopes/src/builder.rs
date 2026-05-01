use crate::problem::Problem;
use collo_ml::problem::ScriptError;
use collo_ml::{ExprType, ExprValue, SemWarning, SqliteDatabaseConnection, SqliteDatabaseDriver};
use collomatique_binding_colloscopes::scripts::SimpleScriptError;
use collomatique_binding_colloscopes::vars::Var;
use collomatique_ilp::ObjectiveSense;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemBuilder {
    inner: collo_ml::problem::ProblemBuilder<SqliteDatabaseDriver, Var>,
}

impl ProblemBuilder {
    pub async fn new(
        modules: &BTreeMap<&str, &str>,
    ) -> Result<Self, ScriptError<SqliteDatabaseConnection>> {
        Ok(ProblemBuilder {
            inner: collo_ml::problem::ProblemBuilder::new(modules).await?,
        })
    }

    pub fn get_warnings(&self) -> &[SemWarning] {
        self.inner.get_warnings()
    }

    pub fn get_fn_signature(
        &self,
        module: &str,
        fn_name: &str,
    ) -> Option<(Vec<ExprType>, ExprType)> {
        self.inner.get_fn_signature(module, fn_name)
    }

    pub fn get_fn_from_module(&self, module: &str) -> BTreeMap<String, (Vec<ExprType>, ExprType)> {
        self.inner.get_fn_from_module(module)
    }

    pub fn add_constraint(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<SqliteDatabaseConnection>>,
    ) -> Result<(), ScriptError<SqliteDatabaseConnection>> {
        self.inner.add_constraint(module, fn_name, args)
    }

    pub fn add_objective(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<SqliteDatabaseConnection>>,
        coefficient: f64,
        sense: ObjectiveSense,
    ) -> Result<(), ScriptError<SqliteDatabaseConnection>> {
        self.inner
            .add_objective(module, fn_name, args, coefficient, sense)
    }

    pub async fn build<Db>(
        self,
        db: &Db,
        db_connection: Option<SqliteDatabaseConnection>,
    ) -> Result<Problem, ScriptError<SqliteDatabaseConnection>>
    where
        <Var as collomatique_ilp_modeler::DescribeVar>::Env:
            collomatique_ilp_modeler::LoadEnv<Db> + Send + Sync + 'static,
        Db: Sync,
    {
        self.inner
            .build(db, db_connection)
            .await
            .map(Problem::from_inner)
    }
}

pub async fn default_problem_builder(
    main_module: &str,
) -> Result<ProblemBuilder, SimpleScriptError> {
    collomatique_binding_colloscopes::scripts::default_problem_builder::<SqliteDatabaseDriver>(
        main_module,
    )
    .await
    .map(|inner| ProblemBuilder { inner })
}
