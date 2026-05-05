//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;
use std::time::Instant;

#[derive(clap::ValueEnum, Clone, Debug)]
enum DebugMode {
    CheckerRecon,
    CheckerBlame,
    CheckerSolve,
    FullRecon,
    FullBlame,
    FullSolve,
    Objective,
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
        let t_total = Instant::now();

        let t = Instant::now();
        eprintln!("Loading file: {:?}", file);
        let (data, _caveats) = collomatique_storage::load_data_from_file(&file).await?;
        let inner_data = data.get_inner_data().clone();
        eprintln!("  File loaded in {:.2?}", t.elapsed());

        let t = Instant::now();
        eprintln!("Building ILP model...");
        let pool = sqlx::SqlitePool::connect(":memory:").await?;
        collomatique_sqlite_state::create_schema(&pool).await?;
        collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data).await?;
        let model = collomatique_constraints_colloscopes::build_model(&pool).await;
        eprintln!("  Model built in {:.2?}", t.elapsed());

        match mode {
            DebugMode::CheckerRecon => debug_recon(&model, &inner_data, true),
            DebugMode::FullRecon => debug_recon(&model, &inner_data, false),
            DebugMode::CheckerBlame => debug_blame(&model, &inner_data, true),
            DebugMode::FullBlame => debug_blame(&model, &inner_data, false),
            DebugMode::CheckerSolve => debug_solve(&model, true),
            DebugMode::FullSolve => debug_solve(&model, false),
            DebugMode::Objective => debug_objective(&model, &inner_data),
        }

        eprintln!("Total: {:.2?}", t_total.elapsed());
        Ok(())
    })
}

fn debug_recon(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
    checker: bool,
) {
    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );
    eprintln!("  Config built in {:.2?}", t.elapsed());

    let t = Instant::now();
    eprintln!("Running {label} reconstruction (CBC logging enabled)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let sol = if checker {
        model.checker_solution_from_data(&config_data, &solver)
    } else {
        model.solution_from_data(&config_data, &solver)
    };

    match sol {
        Some(_) => eprintln!("  Reconstruction SUCCEEDED in {:.2?}", t.elapsed()),
        None => eprintln!("  Reconstruction FAILED in {:.2?}", t.elapsed()),
    }
}

fn debug_blame(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
    checker: bool,
) {
    use collomatique_constraints_colloscopes::ConstraintSource;

    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );
    eprintln!("  Config built in {:.2?}", t.elapsed());

    let t = Instant::now();
    eprintln!("Running {label} reconstruction (silent)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(true);
    let sol = if checker {
        model.checker_solution_from_data(&config_data, &solver)
    } else {
        model.solution_from_data(&config_data, &solver)
    };

    let Some(solution) = sol else {
        eprintln!(
            "  Reconstruction failed in {:.2?}. \
             Use '--debug {label}-recon' to diagnose.",
            t.elapsed()
        );
        return;
    };
    eprintln!("  Reconstruction succeeded in {:.2?}", t.elapsed());

    let t = Instant::now();
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
        eprintln!("  All user constraints satisfied ({:.2?})", t.elapsed());
    } else {
        eprintln!(
            "  {} constraint(s) violated ({:.2?}):",
            violations.len(),
            t.elapsed()
        );
        for (i, msg) in violations.iter().enumerate() {
            eprintln!("    [{}] {}", i + 1, msg);
            if i >= 49 {
                eprintln!("    ... ({} more)", violations.len() - 50);
                break;
            }
        }
    }
}

fn debug_solve(model: &collomatique_constraints_colloscopes::ColloscopeModel, checker: bool) {
    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Solving {label} ILP (CBC logging enabled)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let sol = if checker {
        model.solve_checker(&solver)
    } else {
        model.solve(&solver)
    };

    match sol {
        Some(_) => eprintln!("  Solve SUCCEEDED in {:.2?}", t.elapsed()),
        None => eprintln!(
            "  Solve FAILED (no feasible solution) in {:.2?}",
            t.elapsed()
        ),
    }
}

fn debug_objective(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
) {
    let t = Instant::now();
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );
    eprintln!("  Config built in {:.2?}", t.elapsed());

    let t = Instant::now();
    eprintln!("Running full reconstruction (silent)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(true);
    let sol = model.solution_from_data(&config_data, &solver);

    let Some(solution) = sol else {
        eprintln!(
            "  Reconstruction failed in {:.2?}. \
             Use '--debug full-recon' to diagnose.",
            t.elapsed()
        );
        return;
    };
    eprintln!("  Reconstruction succeeded in {:.2?}", t.elapsed());

    let t = Instant::now();
    let value = solution.eval();
    eprintln!("  Objective value: {value} ({:.2?})", t.elapsed());
}
