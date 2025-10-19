//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

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

    let Some((_collo_id, collo)) = data
        .get_inner_data()
        .colloscopes
        .colloscope_map
        .last_key_value()
    else {
        return Err(anyhow!("No colloscope to solve in file"));
    };

    for caveat in caveats {
        println!("Caveat: {:?}", caveat);
    }

    println!("\nBuilding ILP problem...");

    let problem_with_translators =
        collomatique_solver_glue::colloscopes::ColloscopeProblemWithTranslators::from_collo_params(
            &collo.params,
        )
        .expect("Data should be complete for resolution");

    println!("Start resolution...");

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let solution = problem_with_translators
        .problem
        .solve(&solver)
        .map(|x| x.into_solution());

    println!("\n\nPotential solution: {:?}", solution);

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
