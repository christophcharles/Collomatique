use crate::problem::Problem;
use crate::types::{ConstraintDesc, ReifiedVarName};
use collo_ml::script_feeder::{ScriptError, ScriptFeeder};
use collo_ml::{ExprType, ExprValue, SemWarning, SqliteDatabaseConnection, SqliteDatabaseDriver};
use collomatique_binding_colloscopes::scripts::SimpleScriptError;
use collomatique_binding_colloscopes::vars::Var;
use collomatique_ilp::{ObjectiveSense, Variable};
use collomatique_ilp_modeler::Modeler;
use collomatique_ilp_modeler::bundle::ReifyError;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemBuilder {
    feeder: ScriptFeeder<SqliteDatabaseDriver, Var, ReifiedVarName, ConstraintDesc>,
}

impl ProblemBuilder {
    pub async fn new(
        modules: &BTreeMap<&str, &str>,
    ) -> Result<Self, ScriptError<SqliteDatabaseConnection>> {
        Ok(ProblemBuilder {
            feeder: ScriptFeeder::new(modules).await?,
        })
    }

    pub fn get_warnings(&self) -> &[SemWarning] {
        self.feeder.get_warnings()
    }

    pub fn get_fn_signature(
        &self,
        module: &str,
        fn_name: &str,
    ) -> Option<(Vec<ExprType>, ExprType)> {
        self.feeder.get_fn_signature(module, fn_name)
    }

    pub fn get_fn_from_module(&self, module: &str) -> BTreeMap<String, (Vec<ExprType>, ExprType)> {
        self.feeder.get_fn_from_module(module)
    }

    pub fn add_constraint(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<SqliteDatabaseConnection>>,
    ) -> Result<(), ScriptError<SqliteDatabaseConnection>> {
        self.feeder.add_constraint(module, fn_name, args)
    }

    pub fn add_objective(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<SqliteDatabaseConnection>>,
        coefficient: f64,
        sense: ObjectiveSense,
    ) -> Result<(), ScriptError<SqliteDatabaseConnection>> {
        self.feeder
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
        let bundle = self.feeder.build(db_connection).await?;

        type MyModeler<'m, Db> =
            Modeler<'m, Var, ReifiedVarName, ConstraintDesc, Db, ReifyError<Var, ReifiedVarName>>;

        let mut modeler: MyModeler<'_, Db> = Modeler::from_described(db).await;

        let original_var_list: HashMap<Var, Variable> = modeler
            .base_vars()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, desc) in &original_var_list {
            if !desc.is_integer() {
                return Err(ScriptError::NonIntegerVariable(format!("{:?}", name)));
            }
        }

        modeler
            .apply_bundle(bundle.into_general())
            .expect("no duplicate extras");

        let model = modeler
            .build(db)
            .await
            .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e));

        Ok(Problem::from_model(model, original_var_list))
    }
}

pub async fn default_problem_builder(
    main_module: &str,
) -> Result<ProblemBuilder, SimpleScriptError> {
    use collo_ml::eval::CompileError;
    use collo_ml::script_feeder::ScriptError;
    use collomatique_binding_colloscopes::scripts::MODULES;

    let mut modules: BTreeMap<&str, &str> = MODULES.iter().copied().collect();
    modules.insert("main", main_module);

    let mut builder = ProblemBuilder::new(&modules).await.map_err(|e| match e {
        ScriptError::CompileError(compile_error) => match compile_error {
            CompileError::ParsingError(parse_err) => SimpleScriptError::ParsingError(parse_err),
            CompileError::SemanticsError { errors, warnings } => {
                SimpleScriptError::SemanticErrors { errors, warnings }
            }
            other => SimpleScriptError::UnexpectedError(format!("{}", other)),
        },
        other => SimpleScriptError::UnexpectedError(format!("{}", other)),
    })?;

    let functions = builder.get_fn_from_module("main");

    for (fn_name, _) in &functions {
        if fn_name == "constraint" || fn_name.starts_with("constraint_") {
            builder
                .add_constraint("main", fn_name, vec![])
                .map_err(|e| SimpleScriptError::UnexpectedError(format!("{}", e)))?;
        } else if fn_name == "objective" || fn_name.starts_with("objective_") {
            builder
                .add_objective("main", fn_name, vec![], 1.0, ObjectiveSense::Minimize)
                .map_err(|e| SimpleScriptError::UnexpectedError(format!("{}", e)))?;
        }
    }

    Ok(builder)
}
