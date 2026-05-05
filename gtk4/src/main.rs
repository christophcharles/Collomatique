//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Clone, Debug)]
enum DebugMode {
    Reconstruction,
    Solution,
    Solve,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Args {
    /// Ignore all other parameters and run the python engine
    #[arg(long, default_value_t = false)]
    rpc_engine: bool,

    /// Run in debug mode (requires a file argument)
    #[arg(long, value_enum)]
    debug: Option<DebugMode>,

    /// Open Collomatique directly editing a new colloscope
    #[arg(short, long, default_value_t = false)]
    new: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,

    /// Everything after gets passed through to GTK.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    gtk_options: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if args.rpc_engine {
        return collomatique_rpc_engine::run_rpc_engine();
    }

    if let Some(mode) = args.debug {
        let file = args.file.expect("--debug requires a file argument");
        return run_debug(mode, file);
    }

    let payload = collomatique_gtk4::AppInit {
        new: args.new,
        file_name: args.file,
    };

    let program_invocation = std::env::args().next().unwrap();
    let mut gtk_args = vec![program_invocation];
    gtk_args.extend(args.gtk_options.clone());

    let app = RelmApp::new("fr.collomatique.gtk4").with_args(gtk_args);
    app.allow_multiple_instances(true);
    app.run::<AppModel>(payload);

    Ok(())
}

fn run_debug(mode: DebugMode, file: PathBuf) -> Result<(), anyhow::Error> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let t0 = std::time::Instant::now();
        eprintln!("Loading file: {:?}", file);
        let (data, _caveats) = collomatique_storage::load_data_from_file(&file).await?;
        let inner_data = data.get_inner_data().clone();
        eprintln!("  File loaded in {:.2?}", t0.elapsed());

        let t1 = std::time::Instant::now();
        eprintln!("Building ILP problem...");
        let pool = sqlx::SqlitePool::connect(":memory:").await?;
        collomatique_sqlite_state::create_schema(&pool).await?;
        collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data).await?;
        let problem = collomatique_constraints_colloscopes::build_model(&pool).await;
        eprintln!("  ILP problem built in {:.2?}", t1.elapsed());

        match mode {
            DebugMode::Reconstruction => {
                debug_reconstruction(&problem, &inner_data);
            }
            DebugMode::Solution => {
                debug_solution(&problem, &inner_data);
            }
            DebugMode::Solve => {
                debug_solve(&problem);
            }
        }

        Ok(())
    })
}

fn debug_reconstruction(
    problem: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
) {
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );

    eprintln!("Running reconstruction (CBC logging enabled)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let t = std::time::Instant::now();
    let sol = problem.solution_from_data(&config_data, &solver);
    let elapsed = t.elapsed();

    match sol {
        Some(_) => eprintln!("  Reconstruction SUCCEEDED in {:.2?}", elapsed),
        None => eprintln!("  Reconstruction FAILED (returned None) in {:.2?}", elapsed),
    }
}

fn debug_solution(
    problem: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
) {
    use collomatique_constraints_colloscopes::ConstraintSource;

    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );

    eprintln!("Running reconstruction (silent)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(true);
    let t = std::time::Instant::now();
    let sol = problem.solution_from_data(&config_data, &solver);
    let elapsed = t.elapsed();

    let Some(solution) = sol else {
        eprintln!(
            "  Reconstruction failed in {:.2?}. \
             Use '--debug reconstruction' to diagnose.",
            elapsed
        );
        return;
    };

    eprintln!("  Reconstruction succeeded in {:.2?}", elapsed);
    eprintln!("Checking constraint violations...");

    let env = &inner_data.params;
    let violations: Vec<_> = solution
        .blame()
        .filter_map(|(_constraint, desc)| match desc {
            ConstraintSource::User(desc) => Some(desc.user_readable(env)),
            ConstraintSource::DefiningExtra { .. } => None,
        })
        .collect();

    if violations.is_empty() {
        eprintln!("  All user constraints satisfied.");
    } else {
        eprintln!("  {} constraint(s) violated:", violations.len());
        for (i, msg) in violations.iter().enumerate() {
            eprintln!("    [{}] {}", i + 1, msg);
            if i >= 49 {
                eprintln!("    ... ({} more)", violations.len() - 50);
                break;
            }
        }
    }
}

fn debug_solve(problem: &collomatique_constraints_colloscopes::ColloscopeModel) {
    eprintln!("Solving full ILP (CBC logging enabled, no time limit)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let t = std::time::Instant::now();
    let sol = problem.solve(&solver);
    let elapsed = t.elapsed();

    match sol {
        Some(_) => eprintln!("  Solve SUCCEEDED in {:.2?}", elapsed),
        None => eprintln!("  Solve FAILED (no feasible solution) in {:.2?}", elapsed),
    }
}
