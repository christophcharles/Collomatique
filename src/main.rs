use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::num::{NonZeroU32, NonZeroUsize};

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
    /// Exit shell
    Exit,
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
    new_line_needed: bool,
) -> Result<()> {
    use indicatif::MultiProgress;
    use std::time::Duration;

    let style =
        ProgressStyle::with_template("[{elapsed_precise:.dim}] {spinner:.blue} {prefix}{msg}")
            .unwrap();

    let thread_count = match thread_count {
        Some(value) => value.get(),
        None => num_cpus::get(),
    };

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

    print!("Best cost found is: {}", best_cost.0);
    if new_line_needed {
        println!("");
    }

    Ok(())
}

async fn general_command(
    command: GeneralCommand,
    app_state: &mut AppState<sqlite::Store>,
    new_line_needed: bool,
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
            if new_line_needed {
                println!("");
            }
        }
        GeneralCommand::GetMaxInterrogationsPerDay => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let max_interrogations_per_day = general_data.max_interrogations_per_day;
            match max_interrogations_per_day {
                Some(value) => print!("{}", value.get()),
                None => print!("none"),
            }
            if new_line_needed {
                println!("");
            }
        }
        GeneralCommand::GetInterrogationsPerWeekRange => {
            let general_data = app_state.get_backend_logic().general_data_get().await?;
            let interrogations_per_week = general_data.interrogations_per_week;
            match interrogations_per_week {
                Some(value) => print!("{}..{}", value.start, value.end - 1),
                None => print!("none"),
            }
            if new_line_needed {
                println!("");
            }
        }
    }

    Ok(())
}

async fn week_pattern_command(
    _command: WeekPatternCommand,
    _app_state: &mut AppState<sqlite::Store>,
    _new_line_needed: bool,
) -> Result<()> {
    return Err(anyhow!("Week pattern commands not yet implemented"));
}

async fn shell_command(app_state: &mut AppState<sqlite::Store>) -> Result<()> {
    use nu_ansi_term::{Color, Style};
    use reedline::{DefaultHinter, Emacs, FileBackedHistory, Reedline};

    let keybindings = reedline::default_emacs_keybindings();

    let mut rl = Reedline::create()
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

    match shell_command.command {
        ShellCommand::Global(command) => execute_cli_command(command, app_state, true).await?,
        ShellCommand::Extra(extra_command) => match extra_command {
            ShellExtraCommand::Undo => app_state.undo().await?,
            ShellExtraCommand::Redo => app_state.redo().await?,
            ShellExtraCommand::Exit => return Ok(true),
        },
    }

    Ok(false)
}

async fn execute_cli_command(
    command: CliCommand,
    app_state: &mut AppState<sqlite::Store>,
    new_line_needed: bool,
) -> Result<()> {
    match command {
        CliCommand::General { command } => {
            general_command(command, app_state, new_line_needed).await?
        }
        CliCommand::WeekPatterns { command } => {
            week_pattern_command(command, app_state, new_line_needed).await?
        }
        CliCommand::Solve {
            steps,
            thread_count,
        } => solve_command(steps, thread_count, app_state, new_line_needed).await?,
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let logic = Logic::new(connect_db(args.create, args.db.as_path()).await?);
    let mut app_state = AppState::new(logic);

    let Some(command) = args.command else {
        shell_command(&mut app_state).await?;
        return Ok(());
    };

    execute_cli_command(command, &mut app_state, false).await?;

    Ok(())
}
