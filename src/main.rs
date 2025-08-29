use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::num::NonZeroU32;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Create new database - won't override an existing one
    #[arg(short, long, default_value_t = false)]
    create: bool,
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
    /// Command to run on the database
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Act on general parameters of the colloscope
    General {
        #[command(subcommand)]
        command: GeneralCommand,
    },
    /// Try and solve the colloscope
    Solve,
}

#[derive(Debug, Subcommand)]
enum GeneralCommand {
    /// Change the number of weeks in the colloscope
    SetWeekCount {
        /// New week count for the colloscope
        week_count: NonZeroU32,
        /// Force truncating of week_patterns if needed
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Set a maximum number of interrogations per day for each student
    SetMaxInterrogationsPerDay {
        /// New maximum number of interrogations per day for each student
        max_interrogations_per_day: NonZeroU32,
    },
    /// Disable any maximum number of interrogations per day for each student
    DisableMaxInterrogationsPerDay,
    /// Set a maximum number of interrogations per week for each student - will set minimum to 0 if not already set
    SetMaxInterrogationsPerWeek {
        /// New maximum number of interrogations per week for each student
        max_interrogations_per_week: u32,
    },
    /// Set a maximum number of interrogations per week for each student - will set maximum to the same value if not already set
    SetMinInterrogationsPerWeek {
        /// New minimum number of interrogations per week for each student
        min_interrogations_per_week: u32,
    },
    /// Disable any limit on possible numbers of interrogations per week for each student
    DisableInterrogationsPerWeekRange,
    /// Show the number of weeks currently in the colloscope
    GetWeekCount,
    /// Show the maximum number of interrogations per day (show \"none\" if none is set)
    GetMaxInterrogationsPerDay,
    /// Show the possible range of number of interrogations per week (show \"none\" if none is set)
    GetInterrogationsPerWeekRange,
}

use collomatique::backend::sqlite;
use collomatique::backend::Logic;
use collomatique::frontend::state::AppState;

async fn connect_db(create: bool, path: &std::path::Path) -> Result<sqlite::Store> {
    if create {
        Ok(sqlite::Store::new_db(path).await?)
    } else {
        Ok(sqlite::Store::open_db(path).await?)
    }
}

async fn solve_command(app_state: &mut AppState<sqlite::Store>) -> Result<()> {
    let logic = app_state.get_backend_logic();
    let gen_colloscope_translator = logic.gen_colloscope_translator();
    let data = gen_colloscope_translator.build_validated_data().await?;

    let ilp_translator = data.ilp_translator();

    println!("Generating ILP problem...");
    let problem = ilp_translator.problem();

    println!("{}", problem);

    let general_initializer = collomatique::ilp::initializers::Random::with_p(
        collomatique::ilp::random::DefaultRndGen::new(),
        0.01,
    )
    .unwrap();
    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let max_steps = None;
    let retries = 1;
    let incremental_initializer =
        ilp_translator.incremental_initializer(general_initializer, solver, max_steps, retries);
    let random_gen = collomatique::ilp::random::DefaultRndGen::new();

    use collomatique::ilp::initializers::ConfigInitializer;
    let init_config = incremental_initializer.build_init_config(&problem);
    let sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(init_config);

    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let mutation_policy =
        collomatique::ilp::optimizers::NeighbourMutationPolicy::new(random_gen.clone());
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

async fn general_command(
    command: GeneralCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<()> {
    use collomatique::frontend::state::{Operation, UpdateError};

    match command {
        GeneralCommand::SetWeekCount { week_count, force } => {
            if force {
                todo!("Force flag for \"general set-week-count\"");
            }

            if let Err(e) = app_state
                .apply(Operation::GeneralSetWeekCount(week_count))
                .await
            {
                let err = match e {
                    UpdateError::CannotSetWeekCountWeekPatternsNeedTruncating(
                        week_patterns_to_truncate,
                    ) => {
                        let week_patterns = app_state
                            .get_backend_logic()
                            .week_patterns_get_all()
                            .await?;

                        let week_pattern_list = week_patterns_to_truncate.into_iter().map(
                            |week_pattern| week_patterns.get(&week_pattern).expect("Week pattern id should be valid as it is taken from a dependancy")
                                .name.clone()
                        ).collect::<Vec<_>>().join(", ");

                        anyhow!(
                            format!(
                                "Cannot reduce week_count. The following week patterns need truncating: {}\nYou can use the '-f' flag to force truncating.",
                                week_pattern_list
                            )
                        )
                    }
                    UpdateError::InternalError(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
        }
        GeneralCommand::SetMaxInterrogationsPerDay {
            max_interrogations_per_day: max_interrogation_per_day,
        } => {
            if let Err(e) = app_state
                .apply(Operation::GeneralSetMaxInterrogationPerDay(Some(
                    max_interrogation_per_day,
                )))
                .await
            {
                let err = match e {
                    UpdateError::InternalError(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
        }
        GeneralCommand::DisableMaxInterrogationsPerDay => {
            if let Err(e) = app_state
                .apply(Operation::GeneralSetMaxInterrogationPerDay(None))
                .await
            {
                let err = match e {
                    UpdateError::InternalError(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
        }
        GeneralCommand::SetMaxInterrogationsPerWeek {
            max_interrogations_per_week,
        } => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let interrogations_per_week = match general_data.interrogations_per_week {
                Some(value) => value.start..(max_interrogations_per_week + 1),
                None => 0..(max_interrogations_per_week + 1),
            };
            if interrogations_per_week.is_empty() {
                return Err(anyhow!("The maximum number of interrogations per week must be greater than the minimum number"));
            }
            if let Err(e) = app_state
                .apply(Operation::GeneralSetInterrogationPerWeekRange(Some(
                    interrogations_per_week,
                )))
                .await
            {
                let err = match e {
                    UpdateError::InternalError(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
        }
        GeneralCommand::SetMinInterrogationsPerWeek {
            min_interrogations_per_week,
        } => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let interrogations_per_week = match general_data.interrogations_per_week {
                Some(value) => min_interrogations_per_week..value.end,
                None => min_interrogations_per_week..(min_interrogations_per_week + 1),
            };
            if interrogations_per_week.is_empty() {
                return Err(anyhow!("The minimum number of interrogations per week must be less than the maximum number"));
            }
            if let Err(e) = app_state
                .apply(Operation::GeneralSetInterrogationPerWeekRange(Some(
                    interrogations_per_week,
                )))
                .await
            {
                let err = match e {
                    UpdateError::InternalError(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
        }
        GeneralCommand::DisableInterrogationsPerWeekRange => {
            if let Err(e) = app_state
                .apply(Operation::GeneralSetInterrogationPerWeekRange(None))
                .await
            {
                let err = match e {
                    UpdateError::InternalError(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
        }
        GeneralCommand::GetWeekCount => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let week_count = general_data.week_count.get();
            print!("{}", week_count);
        }
        GeneralCommand::GetMaxInterrogationsPerDay => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let max_interrogations_per_day = general_data.max_interrogations_per_day;
            match max_interrogations_per_day {
                Some(value) => print!("{}", value.get()),
                None => print!("none"),
            }
        }
        GeneralCommand::GetInterrogationsPerWeekRange => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let interrogations_per_week = general_data.interrogations_per_week;
            match interrogations_per_week {
                Some(value) => print!("{}..{}", value.start, value.end - 1),
                None => print!("none"),
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let logic = Logic::new(connect_db(args.create, args.db.as_path()).await?);
    let mut app_state = AppState::new(logic);

    match args.command {
        Command::General { command } => general_command(command, &mut app_state).await?,
        Command::Solve => solve_command(&mut app_state).await?,
    }
    Ok(())
}
