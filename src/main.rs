use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Create new database - won't override an existing one
    #[arg(short, long, default_value_t = false)]
    create: bool,
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
}

use collomatique::backend::sqlite;

async fn connect_db(create: bool, path: &std::path::Path) -> Result<sqlite::Store> {
    if create {
        Ok(sqlite::Store::new_db(path).await?)
    } else {
        Ok(sqlite::Store::open_db(path).await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Opening database...");

    let logic =
        collomatique::backend::Logic::new(connect_db(args.create, args.db.as_path()).await?);
    let gen_colloscope_translator = logic.gen_colloscope_translator();
    let data = gen_colloscope_translator.build_validated_data().await?;

    let ilp_translator = data.ilp_translator();

    println!("Generating ILP problem...");
    let problem = ilp_translator
        .problem_builder()
        .eval_fn(collomatique::debuggable!(|x| {
            if !x
                .get(&collomatique::gen::colloscope::Variable::GroupInSlot {
                    subject: 0,
                    slot: 0,
                    group: 0,
                })
                .unwrap()
            {
                100.
            } else {
                0.
            }
        }))
        .build();

    println!("{}", problem);

    let general_initializer = collomatique::ilp::initializers::Random::with_p(
        collomatique::ilp::random::DefaultRndGen::new(),
        0.01,
    )
    .unwrap();
    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let max_steps = None;
    let retries = 20;
    let incremental_initializer =
        ilp_translator.incremental_initializer(general_initializer, solver, max_steps, retries);
    let random_gen = collomatique::ilp::random::DefaultRndGen::new();

    let variable_count = problem.get_variables().len();
    let p = 2. / (variable_count as f64);

    use collomatique::ilp::initializers::ConfigInitializer;
    let init_config = incremental_initializer.build_init_config(&problem);
    let sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(init_config);

    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let mutation_policy =
        collomatique::ilp::optimizers::RandomMutationPolicy::new(random_gen.clone(), p);
    let iterator = sa_optimizer.iterate(solver, random_gen.clone(), mutation_policy);

    for (i, (sol, cost)) in iterator.enumerate() {
        eprintln!(
            "{}: {} - {:?}",
            i,
            cost,
            ilp_translator.read_solution(sol.as_ref())
        );
    }

    Ok(())
}
