use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroUsize};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    after_help = "If no command is provided, an interactive shell is opened."
)]
/// Collomatique cli tool
struct Cli {
    /// Create new database - won't override an existing one
    #[arg(short, long, default_value_t = false)]
    create: bool,
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
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
        /// Number of concurrent threads to solve the colloscope - default is the number of CPU core
        #[arg(short = 'n')]
        thread_count: Option<NonZeroUsize>,
    },
    /// Create, remove or run python script
    Python {
        #[command(subcommand)]
        command: PythonCommand,
    },
}

#[derive(Debug, Parser)]
#[clap(disable_help_flag = true)]
#[command(name = "")]
struct ShellLine {
    /// Command to run on the database
    #[command(subcommand)]
    command: ShellCommand,
}

#[derive(Debug, Subcommand)]
enum ShellCommand {
    #[command(flatten)]
    Global(CliCommand),
    #[command(flatten)]
    Extra(ShellExtraCommand),
}

#[derive(Debug, Subcommand)]
enum ShellExtraCommand {
    /// Undo last command
    Undo,
    /// Redo previously undone command
    Redo,
    PrintHistory,
    /// Exit shell
    Exit,
}

const DEFAULT_OPTIMIZING_STEP_COUNT: usize = 1000;

#[derive(Debug, Subcommand)]
enum GeneralCommand {
    /// Show or modify week-count (the total number of weeks in the colloscope)
    WeekCount {
        #[command(subcommand)]
        command: WeekCountCommand,
    },
    /// Show or modify the maximum number of interrogations per day for a student
    MaxInterrogationsPerDay {
        #[command(subcommand)]
        command: MaxInterrogationsPerDayCommand,
    },
    /// Show or modify the maximum and minimum number of interrogations per week for a student
    InterrogationsPerWeekRange {
        #[command(subcommand)]
        command: InterrogationsPerWeekRangeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum WeekCountCommand {
    /// Change the number of weeks in the colloscope
    Set {
        /// New week count for the colloscope
        week_count: NonZeroU32,
        /// Force truncating of week_patterns if needed
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Show the number of weeks currently in the colloscope
    Print,
}

#[derive(Debug, Subcommand)]
enum MaxInterrogationsPerDayCommand {
    /// Set a maximum number of interrogations per day for each student
    Set {
        /// New maximum number of interrogations per day for each student
        max_interrogations_per_day: NonZeroU32,
    },
    /// Disable any maximum number of interrogations per day for each student
    Disable,
    /// Show the maximum number of interrogations per day (show \"none\" if none is set)
    Print,
}

#[derive(Debug, Subcommand)]
enum InterrogationsPerWeekRangeCommand {
    /// Set a maximum number of interrogations per week for each student - will set minimum to 0 if not already set
    SetMax {
        /// New maximum number of interrogations per week for each student
        max_interrogations_per_week: u32,
    },
    /// Set a maximum number of interrogations per week for each student - will set maximum to the same value if not already set
    SetMin {
        /// New minimum number of interrogations per week for each student
        min_interrogations_per_week: u32,
    },
    /// Disable any limit on possible numbers of interrogations per week for each student
    Disable,
    /// Show the possible range of number of interrogations per week (show \"none\" if none is set)
    Print,
}

#[derive(Debug, Subcommand)]
enum WeekPatternCommand {
    /// Create a new week pattern
    Create {
        /// Name for the new week pattern
        name: String,
        /// Possible predefined patterns
        pattern: Option<WeekPatternFilling>,
        /// Force creating a new week pattern with an existing name
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Remove an existing week pattern
    Remove {
        /// Name of the week pattern to remove
        name: String,
        /// If multiple week patterns have the same name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
        /// Force removing week pattern even if some data depends on it (the corresponding data will be lost)
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Rename an existing week pattern
    Rename {
        /// Old name of the week pattern
        old_name: String,
        /// If multiple week patterns have the same (old) name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
        /// New name for the week pattern
        new_name: String,
        /// Force using an existing name
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Show all week patterns
    PrintAll,
    /// Show a particular week pattern
    Print {
        /// Name of the week pattern to show
        name: String,
        /// If multiple week patterns have the same name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
    },
    /// Fill existing week pattern with predefined pattern
    Fill {
        /// Name of the week pattern
        name: String,
        /// If multiple week patterns have the same name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
        /// Possible predefined patterns
        pattern: WeekPatternFilling,
    },
    /// Clear existing week pattern to make it empty
    Clear {
        /// Name of the week pattern
        name: String,
        /// If multiple week patterns have the same name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
    },
    /// Add weeks to existing week pattern
    AddWeeks {
        /// Name of the week pattern
        name: String,
        /// If multiple week patterns have the same name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
        /// List of weeks to add separated by spaces (weeks already present in pattern are ignored)
        weeks: Vec<NonZeroU32>,
    },
    /// Delete weeks from existing week pattern
    DeleteWeeks {
        /// Name of the week pattern
        name: String,
        /// If multiple week patterns have the same name, select which one to use.
        /// So if there are 3 week patterns with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        week_pattern_number: Option<NonZeroUsize>,
        /// List of weeks to remove separated by spaces (weeks not in pattern are ignored)
        weeks: Vec<NonZeroU32>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum WeekPatternFilling {
    /// Fill the week pattern with every week for 1 to week_count
    All,
    /// Fill the week pattern with every even week for 2 to week_count
    Even,
    /// Fill the week pattern with every odd week for 1 to week_count
    Odd,
}

#[derive(Debug, Subcommand)]
enum PythonCommand {
    /// Add new python script into the database
    Create {
        /// Name for the python script
        name: String,
        /// File to load the python script from
        file: PathBuf,
        /// Optional function to run in the file
        #[arg(long)]
        func: Option<String>,
        /// Force creating a new python scipt with an existing name
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Delete python script from the database
    Remove {
        /// Name of the python script to remove
        name: String,
        /// If multiple python scripts have the same name, select which one to use.
        /// So if there are 3 python script with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        python_script_number: Option<NonZeroUsize>,
    },
    /// Run a python script
    Run {
        /// Name of the python script to run
        name: String,
        /// Optional csv file to give as input to the python script
        #[arg(long)]
        csv: Option<PathBuf>,
        /// The csv file does not have headers
        #[arg(long)]
        no_headers: bool,
        /// Delimiter for the csv file (default is adjusted for pronote files)
        #[arg(short, long, default_value_t = ';')]
        delimiter: char,
        /// If multiple python scripts have the same name, select which one to use.
        /// So if there are 3 python script with the same name, 1 would refer to the first one, 2 to the second, etc...
        /// Be careful the order might change between databases update (even when using undo/redo)
        #[arg(short = 'n')]
        python_script_number: Option<NonZeroUsize>,
    },
    /// Run a python script directly from a file
    RunFromFile {
        /// Python file to run
        script: PathBuf,
        /// Optional function to run in the file
        #[arg(long)]
        func: Option<String>,
        /// Optional csv file to give as input to the python script
        #[arg(long)]
        csv: Option<PathBuf>,
        /// The csv file does not have headers
        #[arg(long)]
        no_headers: bool,
        /// Delimiter for the csv file (default is adjusted for pronote files)
        #[arg(short, long, default_value_t = ';')]
        delimiter: char,
    },
}

use collomatique::backend::sqlite;
use collomatique::backend::Logic;
use collomatique::frontend::state::{AppSession, AppState};

async fn connect_db(create: bool, path: &std::path::Path) -> Result<sqlite::Store> {
    if create {
        Ok(sqlite::Store::new_db(path).await?)
    } else {
        Ok(sqlite::Store::open_db(path).await?)
    }
}

use collomatique::gen::colloscope::{IlpTranslator, Variable};
use collomatique::ilp::{Config, Problem};
fn solve_initial_guess<'a>(
    ilp_translator: &IlpTranslator<'a>,
    problem: &'a Problem<Variable>,
) -> Config<'a, Variable> {
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

    use collomatique::ilp::initializers::ConfigInitializer;
    incremental_initializer.build_init_config(&problem)
}

use collomatique::ilp::FeasableConfig;
use std::rc::Rc;
fn solve_initial_colloscope<'a>(
    init_config: Config<'a, Variable>,
) -> (
    Option<(Rc<FeasableConfig<'a, Variable>>, f64)>,
    impl Iterator<Item = (Rc<FeasableConfig<'a, Variable>>, f64)>,
) {
    let sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(init_config);
    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let random_gen = collomatique::ilp::random::DefaultRndGen::new();
    let mutation_policy =
        collomatique::ilp::optimizers::NeighbourMutationPolicy::new(random_gen.clone());
    let mut iterator = sa_optimizer.iterate(solver, random_gen, mutation_policy);
    let first_opt = iterator.next();

    (first_opt, iterator)
}

use indicatif::{ProgressBar, ProgressStyle};
fn solve_thread<'a>(
    pb: ProgressBar,
    style: ProgressStyle,
    ilp_translator: &IlpTranslator<'a>,
    problem: &'a Problem<Variable>,
    step_count: usize,
) -> Option<f64> {
    use std::time::Duration;

    pb.set_style(style.clone());
    pb.set_message("Building initial guess... (this can take a few minutes)");
    pb.enable_steady_tick(Duration::from_millis(100));

    let init_config = solve_initial_guess(ilp_translator, problem);

    pb.set_message("Building initial colloscope... (this can take a few minutes)");
    let (first_config_opt, iterator) = solve_initial_colloscope(init_config);

    let (_first_sol, first_cost) = match first_config_opt {
        Some(value) => value,
        None => return None,
    };

    if step_count == 0 {
        pb.finish_with_message(format!("Done. Found colloscope of cost {}", first_cost));
        return Some(first_cost);
    }

    let width = step_count.to_string().len();
    let template = format!(
        "[{{elapsed_precise:.dim}}] {{spinner:.blue}} {{prefix}}Optimizing colloscope... [{{bar:25.green}}] {{pos:>{width}}}/{{len}} - {{wide_msg:!}} {{eta:.dim}}"
    );
    let style = ProgressStyle::with_template(&template)
        .unwrap()
        .progress_chars("=> ");

    pb.set_position(0);
    pb.set_length(step_count.try_into().unwrap());
    pb.set_style(style);
    pb.reset_eta();
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
    pb.finish_with_message(format!("Done. Best colloscope cost is {}.", min_cost));

    Some(min_cost)
}

async fn solve_command(
    steps: Option<usize>,
    thread_count: Option<NonZeroUsize>,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    use collomatique::frontend::state::Manager;
    use indicatif::MultiProgress;
    use std::time::Duration;

    let style =
        ProgressStyle::with_template("[{elapsed_precise:.dim}] {spinner:.blue} {prefix}{msg}")
            .unwrap();

    let thread_count = match thread_count {
        Some(value) => value.get(),
        None => num_cpus::get(),
    };

    let logic = app_state.get_logic();
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

    let multi_pb = MultiProgress::new();
    let mut threads_data = vec![];
    for i in 0..thread_count {
        let pb = multi_pb.add(ProgressBar::new_spinner());
        let width = thread_count.to_string().len();
        if thread_count > 1 {
            pb.set_prefix(format!("[{:>width$}/{}] ", i + 1, thread_count));
        }

        threads_data.push((pb, style.clone(), ilp_translator.clone()));
    }

    // With multithreading, the gag for cbc might have a race condition
    // We just set it up here for the whole computation
    let stdout_gag = gag::Gag::stdout().unwrap();

    let step_count = steps.unwrap_or(DEFAULT_OPTIMIZING_STEP_COUNT);
    let best_cost_opt = {
        use rayon::prelude::*;

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()?;

        thread_pool.install(|| {
            threads_data
                .into_par_iter()
                .panic_fuse()
                .map(|(pb, style, ilp_translator)| {
                    let best_cost = solve_thread(pb, style, &ilp_translator, &problem, step_count)?;

                    use ordered_float::OrderedFloat;
                    Some(OrderedFloat(best_cost))
                })
                .while_some()
                .min()
        })
    };

    drop(stdout_gag);

    let best_cost = match best_cost_opt {
        Some(value) => value,
        None => return Err(anyhow!("No solution found, colloscope is unfeasable!\nThis means the constraints are incompatible and no colloscope can be built that follows all of them. Relax some constraints and try again.")),
    };

    Ok(Some(format!("Best cost found is: {}", best_cost.0)))
}

async fn week_count_command(
    command: WeekCountCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    use collomatique::frontend::state::{Manager, Operation, UpdateError, WeekPatternsOperation};

    match command {
        WeekCountCommand::Set { week_count, force } => {
            if force {
                let mut session = AppSession::new(app_state);

                let week_patterns = session.week_patterns_get_all().await?;

                for (handle, mut wp) in week_patterns {
                    wp.weeks = wp
                        .weeks
                        .into_iter()
                        .filter(|w| w.get() < week_count.get())
                        .collect();

                    if let Err(e) = session
                        .apply(Operation::WeekPatterns(WeekPatternsOperation::Update(
                            handle, wp,
                        )))
                        .await
                    {
                        session.cancel().await;

                        let err = match e {
                            UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                            UpdateError::WeekNumberTooBig(_week) => {
                                panic!(
                                    "The week pattern should be valid as it was checked beforehand"
                                )
                            }
                            _ => panic!("/!\\ Unexpected error ! {:?}", e),
                        };
                        return Err(err);
                    }
                }

                let mut general_data = match session.general_data_get().await {
                    Ok(data) => data,
                    Err(e) => {
                        session.cancel().await;
                        return Err(e.into());
                    }
                };
                general_data.week_count = week_count;

                if let Err(e) = session.apply(Operation::GeneralData(general_data)).await {
                    session.cancel().await;

                    let err = match e {
                        UpdateError::WeekPatternsNeedTruncating(_week_patterns_to_truncate) => {
                            panic!("Week patterns should not need truncating as they were automatically cut.")
                        }
                        UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                        _ => panic!("/!\\ Unexpected error ! {:?}", e),
                    };
                    return Err(err);
                }

                session.commit();
                return Ok(None);
            }

            let mut general_data = app_state.general_data_get().await?;
            general_data.week_count = week_count;

            if let Err(e) = app_state.apply(Operation::GeneralData(general_data)).await {
                let err = match e {
                    UpdateError::WeekPatternsNeedTruncating(week_patterns_to_truncate) => {
                        let week_patterns = app_state.week_patterns_get_all().await?;

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
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        WeekCountCommand::Print => {
            let general_data = app_state.general_data_get().await?;
            let week_count = general_data.week_count.get();
            Ok(Some(week_count.to_string()))
        }
    }
}

async fn max_interrogations_per_day_command(
    command: MaxInterrogationsPerDayCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    use collomatique::frontend::state::{Manager, Operation, UpdateError};

    match command {
        MaxInterrogationsPerDayCommand::Set {
            max_interrogations_per_day: max_interrogation_per_day,
        } => {
            let mut general_data = app_state.general_data_get().await?;
            general_data.max_interrogations_per_day = Some(max_interrogation_per_day);
            if let Err(e) = app_state.apply(Operation::GeneralData(general_data)).await {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        MaxInterrogationsPerDayCommand::Disable => {
            let mut general_data = app_state.general_data_get().await?;
            general_data.max_interrogations_per_day = None;

            if let Err(e) = app_state.apply(Operation::GeneralData(general_data)).await {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        MaxInterrogationsPerDayCommand::Print => {
            let general_data = app_state.general_data_get().await?;
            let max_interrogations_per_day = general_data.max_interrogations_per_day;
            let output = match max_interrogations_per_day {
                Some(value) => value.get().to_string(),
                None => String::from("none"),
            };
            Ok(Some(output))
        }
    }
}

async fn interrogations_per_week_range_command(
    command: InterrogationsPerWeekRangeCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    use collomatique::frontend::state::{Manager, Operation, UpdateError};

    match command {
        InterrogationsPerWeekRangeCommand::SetMax {
            max_interrogations_per_week,
        } => {
            let general_data = app_state.general_data_get().await?;
            let interrogations_per_week = match general_data.interrogations_per_week {
                Some(value) => value.start..(max_interrogations_per_week + 1),
                None => 0..(max_interrogations_per_week + 1),
            };
            if interrogations_per_week.is_empty() {
                return Err(anyhow!("The maximum number of interrogations per week must be greater than the minimum number"));
            }

            let mut general_data = app_state.general_data_get().await?;
            general_data.interrogations_per_week = Some(interrogations_per_week);

            if let Err(e) = app_state.apply(Operation::GeneralData(general_data)).await {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        InterrogationsPerWeekRangeCommand::SetMin {
            min_interrogations_per_week,
        } => {
            let general_data = app_state.general_data_get().await?;
            let interrogations_per_week = match general_data.interrogations_per_week {
                Some(value) => min_interrogations_per_week..value.end,
                None => min_interrogations_per_week..(min_interrogations_per_week + 1),
            };
            if interrogations_per_week.is_empty() {
                return Err(anyhow!("The minimum number of interrogations per week must be less than the maximum number"));
            }
            let mut general_data = app_state.general_data_get().await?;
            general_data.interrogations_per_week = Some(interrogations_per_week);
            if let Err(e) = app_state.apply(Operation::GeneralData(general_data)).await {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        InterrogationsPerWeekRangeCommand::Disable => {
            let mut general_data = app_state.general_data_get().await?;
            general_data.interrogations_per_week = None;
            if let Err(e) = app_state.apply(Operation::GeneralData(general_data)).await {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        InterrogationsPerWeekRangeCommand::Print => {
            let general_data = app_state.general_data_get().await?;
            let interrogations_per_week = general_data.interrogations_per_week;
            let output = match interrogations_per_week {
                Some(value) => format!("{}..={}", value.start, value.end - 1),
                None => String::from("none"),
            };
            Ok(Some(output))
        }
    }
}

async fn general_command(
    command: GeneralCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    match command {
        GeneralCommand::WeekCount { command } => week_count_command(command, app_state).await,
        GeneralCommand::MaxInterrogationsPerDay { command } => {
            max_interrogations_per_day_command(command, app_state).await
        }
        GeneralCommand::InterrogationsPerWeekRange { command } => {
            interrogations_per_week_range_command(command, app_state).await
        }
    }
}

fn week_pattern_to_string(week_pattern: &collomatique::backend::WeekPattern) -> String {
    week_pattern
        .weeks
        .iter()
        .map(|w| (w.get() + 1).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

async fn get_week_pattern(
    app_state: &mut AppState<sqlite::Store>,
    name: &str,
    week_pattern_number: Option<NonZeroUsize>,
) -> Result<(
    collomatique::frontend::state::WeekPatternHandle,
    collomatique::backend::WeekPattern,
)> {
    use collomatique::frontend::state::Manager;

    let week_patterns = app_state.week_patterns_get_all().await?;

    let relevant_week_patterns: Vec<_> = week_patterns
        .into_iter()
        .filter(|(_handle, week_pattern)| week_pattern.name == name)
        .collect();

    if relevant_week_patterns.is_empty() {
        return Err(anyhow!(format!(
            "No week pattern has the name \"{}\".",
            name
        )));
    }
    if week_pattern_number.is_none() && relevant_week_patterns.len() > 1 {
        return Err(anyhow!(
            format!("Several week patterns have the name \"{}\".\nDisambiguate the call by using the '-n' flag.", name)
        ));
    }

    let num = match week_pattern_number {
        Some(n) => n.get() - 1,
        None => 0,
    };
    let output = relevant_week_patterns.get(num).ok_or(anyhow!(
        "There is less than {} different week patterns with the name \"{}\"",
        num + 1,
        name
    ))?;

    Ok(output.clone())
}

fn predefined_week_pattern_weeks(
    filling: WeekPatternFilling,
    week_count: NonZeroU32,
) -> BTreeSet<collomatique::backend::Week> {
    use collomatique::backend::Week;
    let weeks = (0..week_count.get()).into_iter();
    match filling {
        WeekPatternFilling::All => weeks.map(|w| Week::new(w)).collect(),
        WeekPatternFilling::Odd => weeks.step_by(2).map(|w| Week::new(w)).collect(),
        WeekPatternFilling::Even => weeks.skip(1).step_by(2).map(|w| Week::new(w)).collect(),
    }
}

async fn week_patterns_check_existing_names(
    app_state: &mut AppState<sqlite::Store>,
    name: &str,
) -> Result<()> {
    use collomatique::frontend::state::Manager;

    let week_patterns = app_state.week_patterns_get_all().await?;
    for (_, week_pattern) in &week_patterns {
        if week_pattern.name == name {
            return Err(anyhow!(format!(
                "A week pattern with name \"{}\" already exists",
                name
            )));
        }
    }
    Ok(())
}

async fn week_pattern_command(
    command: WeekPatternCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    use collomatique::backend::WeekPattern;
    use collomatique::frontend::state::{Manager, Operation, UpdateError, WeekPatternsOperation};

    match command {
        WeekPatternCommand::Create {
            name,
            pattern,
            force,
        } => {
            if !force {
                week_patterns_check_existing_names(app_state, &name).await?;
            }
            let general_data = app_state.general_data_get().await?;

            let pattern = WeekPattern {
                name,
                weeks: match pattern {
                    Some(filling) => {
                        predefined_week_pattern_weeks(filling, general_data.week_count)
                    }
                    None => BTreeSet::new(),
                },
            };

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Create(
                    pattern,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekNumberTooBig(_week) => panic!(
                        "The week pattern should be valid as it was constructed automatically"
                    ),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }

            Ok(None)
        }
        WeekPatternCommand::Remove {
            name,
            week_pattern_number,
            force,
        } => {
            use collomatique::backend::IdError;

            let (handle, _week_pattern) =
                get_week_pattern(app_state, &name, week_pattern_number).await?;
            let dependancies = app_state
                .week_patterns_check_can_remove(handle)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => anyhow::Error::from(int_err),
                    IdError::InvalidId(id) => panic!(
                        "Id {:?} should be valid as it was obtained from the backend",
                        id
                    ),
                })?;
            if !dependancies.is_empty() {
                if !force {
                    return Err(anyhow!(
                        "Cannot remove week pattern as some data depends on it."
                    ));
                }
                return Err(anyhow!(
                    "force flag not yet implemented for week-patterns remove."
                ));
            }

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Remove(
                    handle,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekPatternDependanciesRemaining(_dep) => {
                        panic!("Dependanciesfor week-pattern should have been checked already")
                    }
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        WeekPatternCommand::Rename {
            old_name,
            week_pattern_number,
            new_name,
            force,
        } => {
            if !force {
                week_patterns_check_existing_names(app_state, &new_name).await?;
            }

            let (handle, week_pattern) =
                get_week_pattern(app_state, &old_name, week_pattern_number).await?;

            let new_week_pattern = WeekPattern {
                name: new_name,
                weeks: week_pattern.weeks,
            };

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Update(
                    handle,
                    new_week_pattern,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekNumberTooBig(_week) => {
                        panic!("The week pattern should be valid as only its name has changed")
                    }
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        WeekPatternCommand::PrintAll => {
            let week_patterns = app_state.week_patterns_get_all().await?;

            let count = week_patterns.len();
            let width = count.to_string().len();

            let week_pattern_vec: Vec<_> = week_patterns
                .iter()
                .enumerate()
                .map(|(i, (_, week_pattern))| {
                    let weeks = week_pattern_to_string(week_pattern);
                    format!("{:>width$} - {}: {}", i + 1, week_pattern.name, weeks)
                })
                .collect();

            Ok(Some(week_pattern_vec.join("\n")))
        }
        WeekPatternCommand::Print {
            name,
            week_pattern_number,
        } => {
            let (_id, week_pattern) =
                get_week_pattern(app_state, &name, week_pattern_number).await?;
            Ok(Some(week_pattern_to_string(&week_pattern)))
        }
        WeekPatternCommand::Fill {
            name,
            week_pattern_number,
            pattern,
        } => {
            let (handle, _week_pattern) =
                get_week_pattern(app_state, &name, week_pattern_number).await?;

            let general_data = app_state.general_data_get().await?;
            let new_week_pattern = WeekPattern {
                name,
                weeks: predefined_week_pattern_weeks(pattern, general_data.week_count),
            };

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Update(
                    handle,
                    new_week_pattern,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekNumberTooBig(_week) => panic!(
                        "The week pattern should be valid as it was constructed automatically"
                    ),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        WeekPatternCommand::Clear {
            name,
            week_pattern_number,
        } => {
            let (handle, _week_pattern) =
                get_week_pattern(app_state, &name, week_pattern_number).await?;

            let new_week_pattern = WeekPattern {
                name,
                weeks: BTreeSet::new(),
            };

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Update(
                    handle,
                    new_week_pattern,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekNumberTooBig(_week) => panic!(
                        "The week pattern should be valid as it was constructed automatically"
                    ),
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        WeekPatternCommand::AddWeeks {
            name,
            week_pattern_number,
            weeks,
        } => {
            use collomatique::backend::Week;

            let general_data = app_state.general_data_get().await?;
            for week in &weeks {
                if week.get() > general_data.week_count.get() {
                    return Err(anyhow!("The week number {} is invalid as it is bigger than week_count (which is {})", week.get(), general_data.week_count.get()));
                }
            }

            let (handle, mut week_pattern) =
                get_week_pattern(app_state, &name, week_pattern_number).await?;

            week_pattern
                .weeks
                .extend(weeks.into_iter().map(|w| Week::new(w.get() - 1)));

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Update(
                    handle,
                    week_pattern,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekNumberTooBig(_week) => {
                        panic!("The week pattern should be valid as it was checked beforehand")
                    }
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
        WeekPatternCommand::DeleteWeeks {
            name,
            week_pattern_number,
            weeks,
        } => {
            use collomatique::backend::Week;

            let (handle, mut week_pattern) =
                get_week_pattern(app_state, &name, week_pattern_number).await?;

            let weeks_to_remove: BTreeSet<_> =
                weeks.into_iter().map(|w| Week::new(w.get() - 1)).collect();

            week_pattern.weeks = week_pattern
                .weeks
                .difference(&weeks_to_remove)
                .copied()
                .collect();

            if let Err(e) = app_state
                .apply(Operation::WeekPatterns(WeekPatternsOperation::Update(
                    handle,
                    week_pattern,
                )))
                .await
            {
                let err = match e {
                    UpdateError::Internal(int_err) => anyhow::Error::from(int_err),
                    UpdateError::WeekNumberTooBig(_week) => {
                        panic!("The week pattern should be valid as it was checked beforehand")
                    }
                    _ => panic!("/!\\ Unexpected error ! {:?}", e),
                };
                return Err(err);
            }
            Ok(None)
        }
    }
}

async fn python_command(
    command: PythonCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    match command {
        PythonCommand::Create {
            name: _name,
            file: _file,
            func: _func,
            force: _force,
        } => Err(anyhow!("python create command not yet implemented")),
        PythonCommand::Remove {
            name: _name,
            python_script_number: _python_script_number,
        } => Err(anyhow!("python remove command not yet implemented")),
        PythonCommand::Run {
            name: _name,
            csv: _csv,
            no_headers: _no_headers,
            delimiter: _delimiter,
            python_script_number: _python_script_number,
        } => Err(anyhow!("python run command not yet implemented")),
        PythonCommand::RunFromFile {
            script,
            func,
            csv,
            no_headers,
            delimiter,
        } => {
            if let Some(path) = csv {
                let python_code = collomatique::frontend::python::PythonCode::from_file(&script)?;
                let csv_content = collomatique::frontend::csv::Content::from_csv_file(&path)?;

                if !delimiter.is_ascii() {
                    return Err(anyhow!(
                        "Csv delimiter must be encoded as a single byte  ASCII character"
                    ));
                }
                let delimiter_str = delimiter.to_string();

                let params = collomatique::frontend::csv::Params {
                    has_headers: !no_headers,
                    delimiter: delimiter_str.as_bytes()[0],
                };

                let csv_extract = csv_content.extract(&params)?;

                {
                    let mut app_session = AppSession::new(app_state);
                    match func {
                        Some(f) => {
                            if let Err(e) = python_code.run_func_with_csv_file(
                                &mut app_session,
                                &f,
                                csv_extract,
                            ) {
                                app_session.cancel().await;
                                return Err(e.into());
                            }
                        }
                        None => {
                            if let Err(e) =
                                python_code.run_with_csv_file(&mut app_session, csv_extract)
                            {
                                app_session.cancel().await;
                                return Err(e.into());
                            }
                        }
                    }
                    app_session.commit();
                }

                Ok(None)
            } else {
                let python_code = collomatique::frontend::python::PythonCode::from_file(&script)?;

                {
                    let mut app_session = AppSession::new(app_state);
                    match func {
                        Some(f) => {
                            if let Err(e) = python_code.run_func(&mut app_session, &f) {
                                app_session.cancel().await;
                                return Err(e.into());
                            }
                        }
                        None => {
                            if let Err(e) = python_code.run(&mut app_session) {
                                app_session.cancel().await;
                                return Err(e.into());
                            }
                        }
                    }
                    app_session.commit();
                }

                Ok(None)
            }
        }
    }
}

struct ReedCompleter {}

impl reedline::Completer for ReedCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<reedline::Suggestion> {
        use clap::CommandFactory;
        use reedline::Span;
        use std::ffi::OsString;

        let mut cmd = ShellLine::command();
        let args = shlex::Shlex::new(line);
        let mut args = std::iter::once("".to_owned())
            .chain(args)
            .map(OsString::from)
            .collect::<Vec<_>>();
        if line.ends_with(' ') || line.is_empty() {
            args.push(OsString::new());
        }
        let arg_index = args.len() - 1;
        let span = Span::new(pos - args[arg_index].len(), pos);

        let current_dir = std::env::current_dir().ok();
        let current_dir_ref = current_dir.as_ref().map(|x| x.as_path());
        let Ok(candidates) =
            clap_complete::dynamic::complete(&mut cmd, args, arg_index, current_dir_ref)
        else {
            return vec![];
        };

        candidates
            .into_iter()
            .map(|c| reedline::Suggestion {
                value: c.0.to_string_lossy().into_owned(),
                description: c.1.map(|x| x.to_string()),
                style: None,
                extra: None,
                span,
                append_whitespace: true,
            })
            .collect()
    }
}

async fn interactive_shell(app_state: &mut AppState<sqlite::Store>) -> Result<()> {
    use nu_ansi_term::{Color, Style};
    use reedline::{
        DefaultHinter, Emacs, FileBackedHistory, IdeMenu, KeyCode, KeyModifiers, MenuBuilder,
        Reedline, ReedlineEvent, ReedlineMenu,
    };

    let completer = Box::new(ReedCompleter {});
    let completion_menu = Box::new(IdeMenu::default().with_name("completion_menu"));

    let mut keybindings = reedline::default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let mut rl = Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_hinter(Box::new(
            DefaultHinter::default().with_style(Style::new().fg(Color::DarkGray).italic()),
        ))
        .with_edit_mode(Box::new(Emacs::new(keybindings)))
        .with_history(Box::new(FileBackedHistory::new(10000).unwrap()));

    loop {
        match respond(&mut rl, app_state).await {
            Ok(quit) => {
                if quit {
                    break;
                }
            }
            Err(err) => {
                use std::io::Write;
                writeln!(std::io::stderr(), "{err}")?;
                std::io::stderr().flush()?;
            }
        }
    }
    Ok(())
}

async fn respond(
    rl: &mut reedline::Reedline,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<bool> {
    use collomatique::frontend::state::Manager;
    use reedline::{DefaultPrompt, DefaultPromptSegment, Signal};

    let mut prompt = DefaultPrompt::default();
    prompt.left_prompt = DefaultPromptSegment::Basic("collomatique".to_owned());
    prompt.right_prompt = DefaultPromptSegment::Empty;

    let line = match rl.read_line(&prompt) {
        Ok(Signal::Success(buffer)) => buffer,
        Ok(Signal::CtrlC | Signal::CtrlD) => return Ok(true),
        _ => return Err(anyhow!("Failed to read line. Input maybe invalid.")),
    };

    if line.trim().is_empty() {
        return Ok(false);
    }

    let shell_command = match shlex::split(&line) {
        Some(mut value) => {
            value.insert(0, String::from(""));
            match ShellLine::try_parse_from(value.iter().map(String::as_str)) {
                Ok(c) => c,
                Err(e) => return Err(e.into()),
            }
        }
        None => return Err(anyhow!("Invalid input")),
    };

    let output = match shell_command.command {
        ShellCommand::Global(command) => execute_cli_command(command, app_state).await?,
        ShellCommand::Extra(extra_command) => match extra_command {
            ShellExtraCommand::Undo => {
                app_state.undo().await?;
                None
            }
            ShellExtraCommand::Redo => {
                app_state.redo().await?;
                None
            }
            ShellExtraCommand::PrintHistory => {
                Some(format!("{:?}", app_state.get_aggregated_history()))
            }
            ShellExtraCommand::Exit => return Ok(true),
        },
    };
    if let Some(msg) = output {
        println!("{}", msg);
    }

    Ok(false)
}

async fn execute_cli_command(
    command: CliCommand,
    app_state: &mut AppState<sqlite::Store>,
) -> Result<Option<String>> {
    match command {
        CliCommand::General { command } => general_command(command, app_state).await,
        CliCommand::WeekPatterns { command } => week_pattern_command(command, app_state).await,
        CliCommand::Solve {
            steps,
            thread_count,
        } => solve_command(steps, thread_count, app_state).await,
        CliCommand::Python { command } => python_command(command, app_state).await,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let logic = Logic::new(connect_db(args.create, args.db.as_path()).await?);
    let mut app_state = AppState::new(logic);

    let Some(command) = args.command else {
        interactive_shell(&mut app_state).await?;
        return Ok(());
    };

    let output = execute_cli_command(command, &mut app_state).await?;
    if let Some(msg) = output {
        print!("{}", msg);
    }

    Ok(())
}
