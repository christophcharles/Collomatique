use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
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
    /// Create, remove and configure week patterns
    WeekPatterns {
        #[command(subcommand)]
        command: WeekPatternCommand,
    },
    /// Try and solve the colloscope
    Solve {
        /// Numbers of optimizing steps - default is 1000
        #[arg(short, long)]
        steps: Option<usize>,
    },
}

const DEFAULT_OPTIMIZING_STEP_COUNT: usize = 1000;

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

#[derive(Debug, Subcommand)]
enum WeekPatternCommand {
    /// Create a new week pattern
    New {
        /// Name for the new week pattern
        name: String,
        /// Possible prefill of the week pattern
        prefill: Option<WeekPatternPrefill>,
        /// Force creating a new week pattern with an existing name
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum WeekPatternPrefill {
    /// Fill the week pattern with every week for 0 to week_count-1
    All,
    /// Fill the week pattern with every even week for 0 to week_count-1
    Even,
    /// Fill the week pattern with every odd week for 1 to week_count-1
    Odd,
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

async fn solve_command(
    steps: Option<usize>,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::time::Duration;

    let style =
        ProgressStyle::with_template("[{elapsed_precise:.dim}] {spinner:.blue} {msg}").unwrap();

    let logic = app_state.get_backend_logic();
    let gen_colloscope_translator = logic.gen_colloscope_translator();
    let data = gen_colloscope_translator.build_validated_data().await?;

    let ilp_translator = data.ilp_translator();

    let pb = ProgressBar::new_spinner().with_style(style.clone());
    pb.set_message("Generating ILP problem...");
    pb.enable_steady_tick(Duration::from_millis(20));
    //let problem = ilp_translator.problem();
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
    pb.finish();

    let pb = ProgressBar::new_spinner().with_style(style.clone());
    pb.set_message("Building initial guess... (this can take a few minutes)");
    pb.enable_steady_tick(Duration::from_millis(20));
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
    pb.finish();

    let pb = ProgressBar::new_spinner().with_style(style.clone());
    pb.set_message("Building initial colloscope... (this can take a few minutes)");
    pb.enable_steady_tick(Duration::from_millis(20));
    let sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(init_config);
    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let mutation_policy =
        collomatique::ilp::optimizers::NeighbourMutationPolicy::new(random_gen.clone());
    let mut iterator = sa_optimizer.iterate(solver, random_gen.clone(), mutation_policy);
    let first_opt = iterator.next();
    pb.finish();
    let (_first_sol, first_cost) = match first_opt {
        Some(value) => value,
        None => return Err(anyhow!("No solution found, colloscope is unfeasable!\nThis means the constraints are incompatible and no colloscope can be built that follows all of them. Relax some constraints and try again.")),
    };

    let step_count = steps.unwrap_or(DEFAULT_OPTIMIZING_STEP_COUNT);
    if step_count != 0 {
        let style = ProgressStyle::with_template("[{elapsed_precise:.dim}] {spinner:.blue} Optimizing colloscope... [{bar:25.green}] {pos}/{len} - {wide_msg:!}").unwrap()
            .progress_chars("=> ");
        let pb = ProgressBar::new(step_count.try_into().unwrap()).with_style(style);
        pb.enable_steady_tick(Duration::from_millis(20));
        pb.set_message(format!(
            "Cost (current/best): {}/{}",
            first_cost, first_cost
        ));
        let mut min_cost = first_cost;
        for (_sol, cost) in iterator.take(step_count) {
            min_cost = if min_cost > cost { cost } else { min_cost };
            pb.set_message(format!("Cost (current/best): {}/{}", cost, min_cost));
            pb.inc(1);
        }
        pb.finish();
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

async fn week_pattern_command(
    command: WeekPatternCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<()> {
    use collomatique::frontend::state::{Operation, UpdateError};

    todo!("Week pattern commands not yet implemented");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let logic = Logic::new(connect_db(args.create, args.db.as_path()).await?);
    let mut app_state = AppState::new(logic);

    match args.command {
        Command::General { command } => general_command(command, &mut app_state).await?,
        Command::WeekPatterns { command } => week_pattern_command(command, &mut app_state).await?,
        Command::Solve { steps } => solve_command(steps, &mut app_state).await?,
    }
    Ok(())
}
