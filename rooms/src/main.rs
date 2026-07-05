use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Assign rooms to scheduling requests")]
struct Args {
    /// CSV file describing available rooms and their characteristics
    #[arg(required_unless_present = "update_csv")]
    rooms: Option<PathBuf>,

    /// CSV file describing scheduling requests and their constraints
    #[arg(required_unless_present = "update_csv")]
    requests: Option<PathBuf>,

    /// CSV file describing room incompatibilities (optional)
    #[arg(long)]
    incompats: Option<PathBuf>,

    /// Solve only feasibility (no objective optimization)
    #[arg(long, conflicts_with_all = ["check", "fix"])]
    no_objective: bool,

    /// Validate an existing solution from SolSalle/SolPrep columns
    #[arg(long, conflicts_with_all = ["fix", "complete", "no_objective"])]
    check: bool,

    /// Find the closest feasible solution to the one in SolSalle/SolPrep
    #[arg(long, conflicts_with_all = ["check", "complete", "no_objective"])]
    fix: bool,

    /// Complete empty SolSalle/SolPrep assignments, keeping filled ones fixed
    #[arg(long, conflicts_with_all = ["check", "fix"])]
    complete: bool,

    /// Use existing SolSalle/SolPrep as warm start hint for the solver
    #[arg(long, conflicts_with_all = ["check", "fix", "complete", "update_csv"])]
    warm: bool,

    /// Convert an old-format requests CSV to the new format (no solving)
    #[arg(long, value_name = "REQUESTS_CSV", conflicts_with_all = ["check", "fix", "complete", "warm", "no_objective"])]
    update_csv: Option<PathBuf>,

    /// Output CSV file for the solution (defaults to stdout)
    #[arg(long, short)]
    out: Option<PathBuf>,

    /// Solver timeout in minutes (0 = no timeout)
    #[arg(long, default_value_t = 10)]
    timeout: u32,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let mode = if args.update_csv.is_some() {
        collomatique_rooms::SolveMode::UpdateCsv
    } else if args.check {
        collomatique_rooms::SolveMode::Check
    } else if args.fix {
        collomatique_rooms::SolveMode::Fix
    } else if args.complete {
        collomatique_rooms::SolveMode::Complete {
            no_objective: args.no_objective,
        }
    } else {
        collomatique_rooms::SolveMode::Solve {
            no_objective: args.no_objective,
            warm: args.warm,
        }
    };

    let requests_path = args
        .update_csv
        .as_deref()
        .or(args.requests.as_deref())
        .expect("requests path required");

    collomatique_rooms::run(
        args.rooms.as_deref(),
        requests_path,
        args.incompats.as_deref(),
        mode,
        args.out.as_deref(),
        collomatique_rooms_model::Config {
            enforce_period_exhaustions: collomatique_rooms_model::Periods {
                p1: true,
                p2: true,
                p3: true,
            },
            ..Default::default()
        },
        args.timeout,
    )?;
    Ok(())
}
