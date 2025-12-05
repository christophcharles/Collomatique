//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use collomatique_state::traits::Manager;
use relm4::RelmApp;
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Args {
    /// Ignore all other parameters and run the python engine
    #[arg(long, default_value_t = false)]
    rpc_engine: bool,

    /// Open Collomatique directly editing a new colloscope
    #[arg(short, long, default_value_t = false)]
    new: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,

    /// TEMPORARY PARAMETER: do not open the gtk4 gui but launch the resolution of the colloscope
    #[arg(long, default_value_t = false)]
    solve: bool,

    /// Everything after gets passed through to GTK.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    gtk_options: Vec<String>,
}

fn try_solve(file: Option<PathBuf>) -> Result<(), anyhow::Error> {
    use anyhow::anyhow;

    let Some(path) = file else {
        return Err(anyhow!("You must specify a file to open and solve"));
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = rt.block_on(collomatique_storage::load_data_from_file(&path));

    let (data, caveats) = match result {
        Ok(r) => r,
        Err(e) => {
            return Err(anyhow!("Failed to load file: {:?}", e));
        }
    };

    for caveat in caveats {
        println!("Caveat: {:?}", caveat);
    }

    println!("\nBuilding ILP problem...");

    use collomatique_binding_colloscopes::scripts::build_default_problem;
    let env = collomatique_binding_colloscopes::views::Env {
        data,
        ignore_prefill_for_group_lists: BTreeSet::new(),
    };
    let problem = build_default_problem(&env);

    println!("\nSolving ILP problem...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let sol_opt = problem.solve(&solver);
    let Some(sol) = sol_opt else {
        println!("\nNo solution found");
        return Ok(());
    };
    println!("\nSolution found!");
    let config_data = sol.get_data();
    let new_colloscope = collomatique_binding_colloscopes::convert::build_colloscope(
        &env.data.get_inner_data().params,
        &config_data,
    )
    .expect("Config data should be compatible with colloscope parameters");

    println!("\nSaving colloscope...");
    let update_ops = env
        .data
        .get_inner_data()
        .colloscope
        .update_ops(new_colloscope)
        .expect("New and old colloscopes should be compatible");

    use collomatique_state::AppState;
    let mut state = AppState::new(env.data);
    for op in update_ops {
        state
            .apply(collomatique_state_colloscopes::Op::Colloscope(op), ())
            .expect("Op should be valid");
    }
    rt.block_on(async {
        collomatique_storage::save_data_to_file(state.get_data(), &path)
            .await
            .expect("Failed to save file")
    });
    println!("\nDone.");

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if args.rpc_engine {
        return collomatique_rpc_engine::run_rpc_engine();
    }

    if args.solve {
        return try_solve(args.file);
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
