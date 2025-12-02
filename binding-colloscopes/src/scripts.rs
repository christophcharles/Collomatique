use super::{vars::Var, views::ObjectId};
use collo_ml::problem::{Problem, ProblemBuilder, Script};
use collomatique_state_colloscopes::Data;
use std::include_str;

pub const GROUP_COUNT_PER_INTERROGATION: &'static str =
    include_str!("scripts/group_count_per_interrogation.collo-ml");

pub const DEFAULT_CONSTRAINT_LIST: &'static [(&'static str, &'static str)] = &[(
    "group_count_per_interrogation",
    GROUP_COUNT_PER_INTERROGATION,
)];

pub fn build_default_problem(data: &Data) -> Problem<ObjectId, Var> {
    let mut builder = ProblemBuilder::<ObjectId, Var>::new(data)
        .expect("ObjectId, Var and Data should be compatible");
    for (name, script) in DEFAULT_CONSTRAINT_LIST {
        let warnings = builder
            .add_constraints(
                Script {
                    name: name.to_string(),
                    content: script.to_string(),
                },
                vec![("constraint".to_string(), vec![])],
            )
            .expect(&format!("Should compile: {}\n\n{}", name, script));

        if !warnings.is_empty() {
            let warnings_str: Vec<_> = warnings.iter().map(|w| w.to_string()).collect();
            eprintln!("Warnings: {}", warnings_str.join("\n"));
        }
    }
    builder.build()
}
