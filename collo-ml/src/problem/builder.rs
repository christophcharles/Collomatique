//! Problem builder for constructing ILP problems.
//!
//! This module defines:
//! - `ProblemBuilder`: Builder pattern for constructing optimization problems

use super::script_feeder::ScriptFeeder;
use super::solution::Problem;
use super::types::{ReifiedVar, ScriptError};
use crate::database::DatabaseDriver;
use crate::eval::{ExprValue, ExternVar, Origin};
use crate::semantics::ArgsType;
use crate::traits::VarConversionError;
use crate::{EvalVar, ExprType, SemWarning};
use collomatique_ilp::{ObjectiveSense, Variable};
use collomatique_ilp_modeler::Modeler;
use collomatique_ilp_modeler::bundle::ReifyError;
use derivative::Derivative;
use std::collections::{BTreeMap, HashMap};

type E<D> = ReifiedVar<D>;
type C<D> = Option<Origin<D>>;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct ProblemBuilder<
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
> {
    feeder: ScriptFeeder<D, V, E<D::Connection>, C<D::Connection>>,
}

impl<
    D: DatabaseDriver,
    V: EvalVar + for<'b> TryFrom<&'b ExternVar<D::Connection>, Error = VarConversionError>,
> ProblemBuilder<D, V>
{
    pub async fn new(modules: &BTreeMap<&str, &str>) -> Result<Self, ScriptError<D::Connection>> {
        Ok(ProblemBuilder {
            feeder: ScriptFeeder::new(modules).await?,
        })
    }

    pub fn get_warnings(&self) -> &[SemWarning] {
        self.feeder.get_warnings()
    }

    pub fn get_fn_signature(&self, module: &str, fn_name: &str) -> Option<(ArgsType, ExprType)> {
        self.feeder.get_fn_signature(module, fn_name)
    }

    pub fn get_fn_from_module(&self, module: &str) -> BTreeMap<String, (ArgsType, ExprType)> {
        self.feeder.get_fn_from_module(module)
    }

    pub fn add_constraint(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<D::Connection>>,
    ) -> Result<(), ScriptError<D::Connection>> {
        self.feeder.add_constraint(module, fn_name, args)
    }

    pub fn add_objective(
        &mut self,
        module: &str,
        fn_name: &str,
        args: Vec<ExprValue<D::Connection>>,
        coefficient: f64,
        sense: ObjectiveSense,
    ) -> Result<(), ScriptError<D::Connection>> {
        self.feeder
            .add_objective(module, fn_name, args, coefficient, sense)
    }

    pub async fn build<Db>(
        self,
        db: &Db,
        db_connection: Option<D::Connection>,
    ) -> Result<Problem<D::Connection, V>, ScriptError<D::Connection>>
    where
        V: 'static,
        V::Env: collomatique_ilp_modeler::LoadEnv<Db> + Send + Sync + 'static,
        Db: Sync + 'static,
    {
        let bundle = self.feeder.build::<Db>(db_connection).await?;

        type MyModeler<'m, D, V, Db> = Modeler<'m, V, E<D>, C<D>, Db, ReifyError<V, E<D>>>;

        let mut modeler: MyModeler<'_, D::Connection, V, Db> = Modeler::from_described(db).await;

        let original_var_list: HashMap<V, Variable> = modeler
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

        Ok(Problem::new(model, original_var_list))
    }
}
