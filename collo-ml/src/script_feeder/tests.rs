use super::*;

type E = ReifiedVar<SqliteDatabaseConnection>;
type C = Option<Origin<SqliteDatabaseConnection>>;

async fn build_model<V>(
    feeder: ScriptFeeder<SqliteDatabaseDriver, V, E, C>,
    env: &V::Env,
) -> Model<V, E, C>
where
    V: DescribeVar
        + EvalVar
        + for<'b> TryFrom<&'b ExternVar<SqliteDatabaseConnection>, Error = VarConversionError>
        + std::fmt::Debug
        + std::hash::Hash
        + Eq
        + Clone
        + Send
        + Sync
        + 'static,
    V::Env: Clone + Send + Sync + 'static,
{
    let bundle = feeder.build(None).await.unwrap();
    let mut modeler: Modeler<'_, V, E, C, V::Env, ReifyError<V, E>> = Modeler::from_described(env);
    for (_name, desc) in modeler.base_vars() {
        assert!(desc.is_integer());
    }
    modeler
        .apply_bundle(bundle.into_general())
        .expect("no duplicate extras");
    modeler
        .build(env)
        .unwrap_or_else(|e| panic!("model build should succeed: {:?}", e))
}

mod complete_interrogation_scheduling;
mod constraints_and_objectives_at_once;
mod constraints_list;
mod errors;
mod eval_var_fix;
mod reification;
mod simple_constraints;
mod simple_objective;
mod two_objectives;
