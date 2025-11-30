use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn single_constraint_problem() {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        V,
    }

    impl EvalVar for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([("V".to_string(), vec![])])
        }
        fn fix(&self) -> Option<f64> {
            None
        }
        fn vars() -> std::collections::BTreeMap<Self, collomatique_ilp::Variable> {
            BTreeMap::from([(Var::V, collomatique_ilp::Variable::binary())])
        }
    }

    impl TryFrom<&ExternVar<NoObject>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<NoObject>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "V" => Ok(Var::V),
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<_, Var>::new(&env).expect("NoObject and Var should be compatible");

    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "base_constraints".into(),
                content: "pub let f() -> Constraint = $V() === 1;".into(),
            },
            vec![("f".to_string(), vec![])],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    assert_eq!(
        sol.get(ProblemVar::Base(Var::V)),
        Some(1.0),
        "Wrong value for solution!"
    );
}
