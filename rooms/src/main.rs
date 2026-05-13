use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Assign rooms to scheduling requests")]
struct Args {
    /// CSV file describing available rooms and their characteristics
    rooms: PathBuf,

    /// CSV file describing scheduling requests and their constraints
    requests: PathBuf,

    /// CSV file describing room incompatibilities (optional)
    #[arg(long)]
    incompats: Option<PathBuf>,

    /// Solve only feasibility (no objective optimization)
    #[arg(long)]
    no_objective: bool,

    /// Validate an existing solution from SolSalle/SolPrep columns
    #[arg(long)]
    check: bool,

    /// Output CSV file for the solution (defaults to stdout)
    #[arg(long, short)]
    out: Option<PathBuf>,

    /// Solver timeout in minutes (0 = no timeout)
    #[arg(long, default_value_t = 10)]
    timeout: u32,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    collomatique_rooms::run(
        &args.rooms,
        &args.requests,
        args.incompats.as_deref(),
        args.no_objective,
        args.check,
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
