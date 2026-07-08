use std::path::PathBuf;
use std::time::Instant;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DebugMode {
    Help,
    CheckerRecon,
    CheckerBlame,
    CheckerBlameMax,
    CheckerSolve,
    FullRecon,
    FullBlame,
    FullBlameMax,
    FullSolve,
    Objective,
    SubprocessSolve,
    SubprocessSolveStrategy,
    NoObjective,
    NoObjectiveStarter,
    Incremental,
    Conductor,
}

pub fn print_help() -> Result<(), anyhow::Error> {
    eprintln!("Available debug modes:");
    eprintln!();
    eprintln!("  help               Show this help message");
    eprintln!();
    eprintln!("  Checker modes (user constraints + constraint-needed extras only):");
    eprintln!(
        "    checker-recon      Reconstruct extra variables from current colloscope (CBC logging on)"
    );
    eprintln!(
        "    checker-blame      List violated constraints after reconstruction (minimal filtering)"
    );
    eprintln!("    checker-blame-max  List ALL violated constraints without filtering");
    eprintln!("    checker-solve      Solve the checker ILP from scratch (CBC logging on)");
    eprintln!();
    eprintln!("  Full modes (all constraints + objectives):");
    eprintln!("    full-recon         Reconstruct all extra variables (CBC logging on)");
    eprintln!(
        "    full-blame         List violated constraints after full reconstruction (minimal filtering)"
    );
    eprintln!("    full-blame-max     List ALL violated constraints without filtering");
    eprintln!("    full-solve         Solve the full ILP from scratch (CBC logging on)");
    eprintln!();
    eprintln!("  Other:");
    eprintln!(
        "    objective          Compute the objective function value for the current colloscope"
    );
    eprintln!("    subprocess-solve   Solve the full ILP via a subprocess (end-to-end test)");
    eprintln!("    subprocess-solve-strategy  Solve via a strategy subprocess (end-to-end test)");
    eprintln!(
        "    no-objective           Solve via no-objective strategy subprocess (end-to-end test)"
    );
    eprintln!(
        "    no-objective-starter   Solve via no-objective starter strategy subprocess (end-to-end test)"
    );
    eprintln!(
        "    incremental            Solve via the incremental (staggered epochs) strategy subprocess"
    );
    eprintln!(
        "    conductor              Solve via the conductor strategy (spawns subprocess workers)"
    );
    eprintln!();
    eprintln!("  All modes except 'help' require a file argument.");
    eprintln!("  Blame modes use 'minimal' filtering by default: redundant constraints implied");
    eprintln!("  by more specific ones are removed. Use 'blame-max' variants to see everything.");
    Ok(())
}

pub fn run(mode: DebugMode, file: PathBuf) -> Result<(), anyhow::Error> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let t_total = Instant::now();

        let t = Instant::now();
        eprintln!("Loading file: {:?}", file);
        let (data, _caveats) = collomatique_storage::load_data_from_file(&file).await?;
        let inner_data = data.get_inner_data().clone();
        eprintln!("  File loaded in {:.2?}", t.elapsed());

        eprintln!("Building ILP model...");
        let pool = sqlx::SqlitePool::connect(":memory:").await?;
        collomatique_sqlite_state::create_schema(&pool).await?;
        collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data).await?;
        let model = collomatique_constraints_colloscopes::build_model_with_log(&pool, &mut |msg| {
            eprintln!("  {msg}")
        })
        .await;
        let stats = model.stats();
        eprintln!("  Model statistics:");
        eprintln!("    Base variables: {}", stats.base_variable_count);
        eprintln!("    User constraints: {}", stats.user_constraint_count);
        eprintln!(
            "    Constraint extras: {} ({} defining constraints)",
            stats.constraint_extra_count, stats.constraint_defining_constraint_count,
        );
        eprintln!(
            "    Objective extras: {} ({} defining constraints)",
            stats.objective_extra_count, stats.objective_defining_constraint_count,
        );

        match mode {
            DebugMode::Help => unreachable!("handled before file loading"),
            DebugMode::CheckerRecon => recon(&model, &inner_data, true),
            DebugMode::FullRecon => recon(&model, &inner_data, false),
            DebugMode::CheckerBlame => blame(&model, &inner_data, true, true),
            DebugMode::CheckerBlameMax => blame(&model, &inner_data, true, false),
            DebugMode::FullBlame => blame(&model, &inner_data, false, true),
            DebugMode::FullBlameMax => blame(&model, &inner_data, false, false),
            DebugMode::CheckerSolve => solve(&model, true),
            DebugMode::FullSolve => solve(&model, false),
            DebugMode::Objective => objective(&model, &inner_data),
            DebugMode::SubprocessSolve => subprocess_solve(&model),
            DebugMode::SubprocessSolveStrategy => subprocess_solve_strategy(&model),
            DebugMode::NoObjective => no_objective_solve(&model),
            DebugMode::NoObjectiveStarter => no_objective_starter_solve(&model),
            DebugMode::Incremental => incremental_solve(&model),
            DebugMode::Conductor => conductor_solve(&model).await,
        }

        eprintln!("Total: {:.2?}", t_total.elapsed());
        Ok(())
    })
}

fn recon(
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
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(false);
    let sol = if checker {
        model.checker_solution_from_data(&config_data, &solver)
    } else {
        model.solution_from_data(&config_data, &solver)
    };

    match sol {
        Ok(_) => eprintln!("  Reconstruction SUCCEEDED in {:.2?}", t.elapsed()),
        Err(e) => eprintln!("  Reconstruction FAILED in {:.2?}: {e}", t.elapsed()),
    }
}

fn blame(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
    checker: bool,
    minimal: bool,
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
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(true);
    let sol = if checker {
        model.checker_solution_from_data(&config_data, &solver)
    } else {
        model.solution_from_data(&config_data, &solver)
    };

    let Ok(solution) = sol else {
        eprintln!(
            "  Reconstruction failed in {:.2?}. \
             Use '--debug {label}-recon' to diagnose.",
            t.elapsed()
        );
        return;
    };
    eprintln!("  Reconstruction succeeded in {:.2?}", t.elapsed());

    let t = Instant::now();
    let filter_label = if minimal { "minimal" } else { "all" };
    eprintln!("Checking constraint violations ({filter_label})...");
    let env = &inner_data.params;

    use collomatique_constraints_colloscopes::SEVERITY_LEVEL_COUNT;
    let violations: Vec<&collomatique_constraints_colloscopes::ConstraintDesc> = if minimal {
        let mb = solution.minimal_blame();
        mb.iter().copied().collect()
    } else {
        solution
            .blame()
            .filter_map(|(_constraint, desc)| match desc {
                ConstraintSource::User(desc) => Some(desc),
                ConstraintSource::DefiningExtra { .. } => None,
            })
            .collect()
    };

    if violations.is_empty() {
        eprintln!("  All user constraints satisfied ({:.2?})", t.elapsed());
        return;
    }

    eprintln!(
        "  {} constraint(s) violated ({:.2?}):",
        violations.len(),
        t.elapsed()
    );

    let mut buckets: [Vec<&collomatique_constraints_colloscopes::ConstraintDesc>;
        SEVERITY_LEVEL_COUNT] = Default::default();
    for desc in &violations {
        buckets[desc.severity_level() as usize].push(desc);
    }

    const MAX_DETAIL_LINES: usize = 50;
    let mut budget = MAX_DETAIL_LINES;
    let mut global_index: usize = 0;

    for (level, descs) in buckets.iter().enumerate() {
        if descs.is_empty() {
            continue;
        }

        let count = descs.len();
        eprintln!();
        eprintln!(
            "Severity Level {}: {} ({} failure{})",
            level,
            descs[0].severity_label(),
            count,
            if count == 1 { "" } else { "s" },
        );

        if budget == 0 {
            continue;
        }

        eprintln!("---");
        let printable = budget.min(count);
        for desc in &descs[..printable] {
            global_index += 1;
            eprintln!("  [{}] {}", global_index, desc.user_readable(env));
        }
        budget -= printable;

        let remaining = count - printable;
        if remaining > 0 {
            eprintln!("(... and {} more)", remaining);
        }
        global_index += remaining;
    }
}

fn solve(model: &collomatique_constraints_colloscopes::ColloscopeModel, checker: bool) {
    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Solving {label} ILP (CBC logging enabled)...");
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(false);
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

fn objective(
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
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(true);
    let sol = model.solution_from_data(&config_data, &solver);

    let Ok(solution) = sol else {
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

fn subprocess_solve(model: &collomatique_constraints_colloscopes::ColloscopeModel) {
    use collomatique_subprocesses::{IlpSolverConfig, IlpStatus, SolverSubprocess};
    use std::sync::mpsc;

    let t = Instant::now();
    eprintln!("Extracting problem descriptor...");
    let (desc, var_order) = model.problem().get_desc();
    eprintln!(
        "  Descriptor: {} variables, {} constraints ({:.2?})",
        desc.variables.len(),
        desc.constraints.len(),
        t.elapsed()
    );

    let config = IlpSolverConfig {
        problem_desc: desc,
        warm_start: None,
        time_limit_seconds: None,
        disable_logging: false,
    };

    let (tx, rx) = mpsc::channel();

    eprintln!("Spawning solver subprocess...");
    let t = Instant::now();
    let handle = SolverSubprocess::spawn(
        config,
        move |result| {
            let _ = tx.send(result);
        },
        |progress| {
            eprintln!(
                "  [subprocess progress] obj={} bound={:.4} nodes={} solutions={}",
                progress
                    .best_obj
                    .map_or_else(|| "-".to_owned(), |o| format!("{o:.4}")),
                progress.best_bound,
                progress.node_count,
                progress.solutions_found
            );
        },
        |line| {
            eprint!("  [subprocess] {}", line);
        },
    );

    let handle = match handle {
        Ok(h) => h,
        Err(e) => {
            eprintln!("  Failed to spawn subprocess: {}", e);
            return;
        }
    };

    eprintln!("  Subprocess spawned in {:.2?}", t.elapsed());
    eprintln!("Waiting for result...");

    let t = Instant::now();
    let result = rx.recv();
    match result {
        Ok(result) => {
            eprintln!("  Result received in {:.2?}", t.elapsed());
            eprintln!("  Status: {:?}", result.status);
            match result.obj_value {
                Some(v) => eprintln!("  Objective: {}", v),
                None => eprintln!("  Objective: N/A"),
            }
            match result.best_bound {
                Some(v) => eprintln!("  Best bound: {}", v),
                None => eprintln!("  Best bound: N/A"),
            }
            eprintln!("  Nodes: {}", result.node_count);

            if let Some(ref solution) = result.solution {
                eprintln!("  Solution has {} values", solution.len());
                let config_data = collomatique_ilp::solution_to_config_data(solution, &var_order);
                let problem = model.problem();
                match problem.build_config(config_data) {
                    Ok(config) => {
                        if config.is_feasible() {
                            eprintln!("  Solution is FEASIBLE");
                        } else {
                            let violated = config.blame().len();
                            eprintln!("  Solution violates {} constraint(s)", violated);
                        }
                    }
                    Err(check) => {
                        eprintln!(
                            "  Config build failed: missing={}, excess={}, non_conforming={}",
                            check.missing_variables.len(),
                            check.excess_variables.len(),
                            check.non_conforming_variables.len(),
                        );
                    }
                }
            } else {
                eprintln!("  No solution returned");
            }

            if result.status == IlpStatus::Error {
                eprintln!("  Solver reported an error");
            }
        }
        Err(_) => {
            eprintln!("  Channel closed without receiving a result");
            if let Some(progress) = handle.last_progress() {
                eprintln!(
                    "  Last progress: obj={}, bound={}, nodes={}, solutions={}",
                    progress
                        .best_obj
                        .map_or_else(|| "-".to_owned(), |o| format!("{o}")),
                    progress.best_bound,
                    progress.node_count,
                    progress.solutions_found
                );
            }
        }
    }
}

fn subprocess_solve_strategy(model: &collomatique_constraints_colloscopes::ColloscopeModel) {
    use collomatique_strategies::{
        DefaultPayload, DefaultStrategy, SolveProgress, SolveStatus, StrategyOutcome,
        StrategyProgressData,
    };
    use collomatique_subprocesses::StrategySubprocess;
    use std::sync::mpsc;

    type Outcome = StrategyOutcome<
        collomatique_ilp_modeler::InternalVar<
            collomatique_constraints_colloscopes::Var,
            collomatique_constraints_colloscopes::ExtraVarName,
        >,
    >;

    let t = Instant::now();
    eprintln!("Extracting problem descriptor...");
    let (model_desc, _) = model.to_desc();
    eprintln!(
        "  Descriptor: {} variables, {} constraints ({:.2?})",
        model_desc.main.problem_desc.variables.len(),
        model_desc.main.problem_desc.constraints.len(),
        t.elapsed()
    );

    let strategy = DefaultStrategy {
        time_limit_seconds: None,
        disable_logging: false,
    };

    let (tx, rx) = mpsc::channel();
    eprintln!("Spawning strategy subprocess...");
    let t = Instant::now();
    let handle = StrategySubprocess::spawn(
        model,
        &strategy,
        None,
        DefaultPayload::default(),
        move |outcome: Outcome| {
            let _ = tx.send(outcome);
        },
        |progress: Result<SolveProgress<_>, String>| match progress {
            Ok(p) => {
                eprintln!("  [strategy subprocess progress] {p}");
            }
            Err(e) => {
                eprintln!("  [strategy subprocess progress error] {e}");
            }
        },
        |line| {
            eprint!("  [strategy subprocess] {}", line);
        },
    );

    let handle = match handle {
        Ok(h) => h,
        Err(e) => {
            eprintln!("  Failed to spawn strategy subprocess: {}", e);
            return;
        }
    };

    eprintln!("  Strategy subprocess spawned in {:.2?}", t.elapsed());
    eprintln!("Waiting for strategy result...");

    let t = Instant::now();
    let result = rx.recv();
    match result {
        Ok(outcome) => {
            eprintln!("  Result received in {:.2?}", t.elapsed());
            eprintln!("  Status: {:?}", outcome.status);
            match outcome.objective {
                Some(v) => eprintln!("  Objective: {}", v),
                None => eprintln!("  Objective: N/A"),
            }
            match outcome.best_bound {
                Some(v) => eprintln!("  Best bound: {}", v),
                None => eprintln!("  Best bound: N/A"),
            }

            if let Some(ref config_data) = outcome.solution {
                let problem = model.problem();
                match problem.build_config(config_data.clone()) {
                    Ok(config) => {
                        if config.is_feasible() {
                            eprintln!("  Solution is FEASIBLE");
                        } else {
                            let violated = config.blame().len();
                            eprintln!("  Solution violates {} constraint(s)", violated);
                        }
                    }
                    Err(check) => {
                        eprintln!(
                            "  Config build failed: missing={}, excess={}, non_conforming={}",
                            check.missing_variables.len(),
                            check.excess_variables.len(),
                            check.non_conforming_variables.len(),
                        );
                    }
                }
            } else {
                eprintln!("  No solution returned");
            }

            if outcome.status == SolveStatus::Error {
                eprintln!("  Strategy reported an error");
            }
        }
        Err(_) => {
            eprintln!("  Channel closed without receiving a result");
            if let Some(StrategyProgressData::Default(p)) = handle.last_progress() {
                eprintln!("  Last progress: {p}");
            }
        }
    }
}

fn no_objective_solve(model: &collomatique_constraints_colloscopes::ColloscopeModel) {
    use collomatique_strategies::{
        NoObjectivePayload, NoObjectiveProgressData, NoObjectiveStrategy, SolveStatus,
        StrategyOutcome, StrategyProgressData,
    };
    use collomatique_subprocesses::StrategySubprocess;
    use std::sync::mpsc;

    type Outcome = StrategyOutcome<
        collomatique_ilp_modeler::InternalVar<
            collomatique_constraints_colloscopes::Var,
            collomatique_constraints_colloscopes::ExtraVarName,
        >,
    >;

    let t = Instant::now();
    eprintln!("Extracting problem descriptor...");
    let (model_desc, _) = model.to_desc();
    eprintln!(
        "  Descriptor: {} variables, {} constraints ({:.2?})",
        model_desc.main.problem_desc.variables.len(),
        model_desc.main.problem_desc.constraints.len(),
        t.elapsed()
    );

    let strategy = NoObjectiveStrategy {
        checker_time_limit_seconds: None,
        reconstruction_time_limit_seconds: None,
        disable_logging: false,
    };

    let (tx, rx) = mpsc::channel();
    eprintln!("Spawning no-objective strategy subprocess...");
    let t = Instant::now();
    let handle = StrategySubprocess::spawn(
        model,
        &strategy,
        None,
        NoObjectivePayload::default(),
        move |outcome: Outcome| {
            let _ = tx.send(outcome);
        },
        |progress: Result<NoObjectiveProgressData, String>| match progress {
            Ok(p) => {
                eprintln!("  [strategy subprocess progress] {p}");
            }
            Err(e) => {
                eprintln!("  [strategy subprocess progress error] {e}");
            }
        },
        |line| {
            eprint!("  [strategy subprocess] {}", line);
        },
    );

    let handle = match handle {
        Ok(h) => h,
        Err(e) => {
            eprintln!("  Failed to spawn strategy subprocess: {}", e);
            return;
        }
    };

    eprintln!("  Strategy subprocess spawned in {:.2?}", t.elapsed());
    eprintln!("Waiting for strategy result...");

    let t = Instant::now();
    let result = rx.recv();
    match result {
        Ok(outcome) => {
            eprintln!("  Result received in {:.2?}", t.elapsed());
            eprintln!("  Status: {:?}", outcome.status);
            match outcome.objective {
                Some(v) => eprintln!("  Objective: {}", v),
                None => eprintln!("  Objective: N/A"),
            }
            match outcome.best_bound {
                Some(v) => eprintln!("  Best bound: {}", v),
                None => eprintln!("  Best bound: N/A"),
            }

            if let Some(ref config_data) = outcome.solution {
                let problem = model.problem();
                match problem.build_config(config_data.clone()) {
                    Ok(config) => {
                        if config.is_feasible() {
                            eprintln!("  Solution is FEASIBLE");
                        } else {
                            let violated = config.blame().len();
                            eprintln!("  Solution violates {} constraint(s)", violated);
                        }
                    }
                    Err(check) => {
                        eprintln!(
                            "  Config build failed: missing={}, excess={}, non_conforming={}",
                            check.missing_variables.len(),
                            check.excess_variables.len(),
                            check.non_conforming_variables.len(),
                        );
                    }
                }
            } else {
                eprintln!("  No solution returned");
            }

            if outcome.status == SolveStatus::Error {
                eprintln!("  Strategy reported an error");
            }
        }
        Err(_) => {
            eprintln!("  Channel closed without receiving a result");
            if let Some(StrategyProgressData::NoObjective(p)) = handle.last_progress() {
                eprintln!("  Last progress: {p}");
            }
        }
    }
}

fn no_objective_starter_solve(model: &collomatique_constraints_colloscopes::ColloscopeModel) {
    use collomatique_strategies::{
        DefaultStrategy, NoObjectiveStarterPayload, NoObjectiveStarterProgress,
        NoObjectiveStarterStrategy, NoObjectiveStrategy, SolveStatus, StrategyOutcome,
    };
    use collomatique_subprocesses::StrategySubprocess;
    use std::sync::mpsc;

    type Outcome = StrategyOutcome<
        collomatique_ilp_modeler::InternalVar<
            collomatique_constraints_colloscopes::Var,
            collomatique_constraints_colloscopes::ExtraVarName,
        >,
    >;

    type V = collomatique_ilp_modeler::InternalVar<
        collomatique_constraints_colloscopes::Var,
        collomatique_constraints_colloscopes::ExtraVarName,
    >;

    let t = Instant::now();
    eprintln!("Extracting problem descriptor...");
    let (model_desc, _) = model.to_desc();
    eprintln!(
        "  Descriptor: {} variables, {} constraints ({:.2?})",
        model_desc.main.problem_desc.variables.len(),
        model_desc.main.problem_desc.constraints.len(),
        t.elapsed()
    );

    let strategy = NoObjectiveStarterStrategy {
        no_objective: NoObjectiveStrategy {
            checker_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: false,
        },
        default: DefaultStrategy {
            time_limit_seconds: None,
            disable_logging: false,
        },
    };

    let (tx, rx) = mpsc::channel();
    eprintln!("Spawning no-objective-starter strategy subprocess...");
    let t = Instant::now();
    let handle = StrategySubprocess::spawn(
        model,
        &strategy,
        None,
        NoObjectiveStarterPayload::default(),
        move |outcome: Outcome| {
            let _ = tx.send(outcome);
        },
        |progress: Result<NoObjectiveStarterProgress<V>, String>| match progress {
            Ok(NoObjectiveStarterProgress::HintFound { .. }) => {
                eprintln!("  [strategy subprocess progress] Hint found!");
            }
            Ok(p) => {
                eprintln!("  [strategy subprocess progress] {p}");
            }
            Err(e) => {
                eprintln!("  [strategy subprocess progress error] {e}");
            }
        },
        |line| {
            eprint!("  [strategy subprocess] {}", line);
        },
    );

    let handle = match handle {
        Ok(h) => h,
        Err(e) => {
            eprintln!("  Failed to spawn strategy subprocess: {}", e);
            return;
        }
    };

    eprintln!("  Strategy subprocess spawned in {:.2?}", t.elapsed());
    eprintln!("Waiting for strategy result...");

    let t = Instant::now();
    let result = rx.recv();
    match result {
        Ok(outcome) => {
            eprintln!("  Result received in {:.2?}", t.elapsed());
            eprintln!("  Status: {:?}", outcome.status);
            match outcome.objective {
                Some(v) => eprintln!("  Objective: {}", v),
                None => eprintln!("  Objective: N/A"),
            }
            match outcome.best_bound {
                Some(v) => eprintln!("  Best bound: {}", v),
                None => eprintln!("  Best bound: N/A"),
            }

            if let Some(ref config_data) = outcome.solution {
                let problem = model.problem();
                match problem.build_config(config_data.clone()) {
                    Ok(config) => {
                        if config.is_feasible() {
                            eprintln!("  Solution is FEASIBLE");
                        } else {
                            let violated = config.blame().len();
                            eprintln!("  Solution violates {} constraint(s)", violated);
                        }
                    }
                    Err(check) => {
                        eprintln!(
                            "  Config build failed: missing={}, excess={}, non_conforming={}",
                            check.missing_variables.len(),
                            check.excess_variables.len(),
                            check.non_conforming_variables.len(),
                        );
                    }
                }
            } else {
                eprintln!("  No solution returned");
            }

            if outcome.status == SolveStatus::Error {
                eprintln!("  Strategy reported an error");
            }
        }
        Err(_) => {
            eprintln!("  Channel closed without receiving a result");
            if let Some(progress) = handle.last_progress() {
                eprintln!("  Last progress: {progress}");
            }
        }
    }
}

fn incremental_solve(model: &collomatique_constraints_colloscopes::ColloscopeModel) {
    use collomatique_constraints_colloscopes::Var;
    use collomatique_ilp_modeler::InternalVar;
    use collomatique_strategies::{
        IncrementalPayload, IncrementalProgressData, IncrementalStrategy, SolveStatus,
        StrategyOutcome, StrategyProgressData,
    };
    use collomatique_subprocesses::StrategySubprocess;
    use std::collections::HashMap;
    use std::sync::mpsc;

    type Outcome = StrategyOutcome<
        collomatique_ilp_modeler::InternalVar<
            collomatique_constraints_colloscopes::Var,
            collomatique_constraints_colloscopes::ExtraVarName,
        >,
    >;

    let t = Instant::now();
    eprintln!("Extracting problem descriptor...");
    let (model_desc, _) = model.to_desc();
    eprintln!(
        "  Descriptor: {} variables, {} constraints ({:.2?})",
        model_desc.main.problem_desc.variables.len(),
        model_desc.main.problem_desc.constraints.len(),
        t.elapsed()
    );

    // Epoch assignment: every StudentGroup base variable is solved first (epoch 0), then each
    // GroupInInterrogation variable is solved in the epoch matching its week (week + 1), so the
    // schedule fills in week by week on top of the fixed group assignment.
    let mut epochs = HashMap::new();
    for v in model.problem().get_variables().keys() {
        if let InternalVar::Base(base) = v {
            let epoch = match base {
                Var::StudentGroup { .. } => 0u32,
                Var::GroupInInterrogation { week, .. } => week.0 as u32 + 1,
            };
            epochs.insert(v.clone(), epoch);
        }
    }
    eprintln!(
        "  Epoch payload: {} base variables across {} epoch(s)",
        epochs.len(),
        epochs.values().copied().max().map_or(0, |m| m + 1),
    );
    let payload = IncrementalPayload { epochs };

    let strategy = IncrementalStrategy {
        l1_weight: 1000.0,
        epoch_time_limit_seconds: None,
        reconstruction_time_limit_seconds: None,
        disable_logging: false,
    };

    let (tx, rx) = mpsc::channel();
    eprintln!("Spawning incremental strategy subprocess...");
    let t = Instant::now();
    let handle = StrategySubprocess::spawn(
        model,
        &strategy,
        None,
        payload,
        move |outcome: Outcome| {
            let _ = tx.send(outcome);
        },
        |progress: Result<IncrementalProgressData, String>| match progress {
            Ok(p) => {
                eprintln!("  [strategy subprocess progress] {p}");
            }
            Err(e) => {
                eprintln!("  [strategy subprocess progress error] {e}");
            }
        },
        |line| {
            eprint!("  [strategy subprocess] {}", line);
        },
    );

    let handle = match handle {
        Ok(h) => h,
        Err(e) => {
            eprintln!("  Failed to spawn strategy subprocess: {}", e);
            return;
        }
    };

    eprintln!("  Strategy subprocess spawned in {:.2?}", t.elapsed());
    eprintln!("Waiting for strategy result...");

    let t = Instant::now();
    let result = rx.recv();
    match result {
        Ok(outcome) => {
            eprintln!("  Result received in {:.2?}", t.elapsed());
            eprintln!("  Status: {:?}", outcome.status);
            match outcome.objective {
                Some(v) => eprintln!("  Objective: {}", v),
                None => eprintln!("  Objective: N/A"),
            }
            match outcome.best_bound {
                Some(v) => eprintln!("  Best bound: {}", v),
                None => eprintln!("  Best bound: N/A"),
            }

            if let Some(ref config_data) = outcome.solution {
                let problem = model.problem();
                match problem.build_config(config_data.clone()) {
                    Ok(config) => {
                        if config.is_feasible() {
                            eprintln!("  Solution is FEASIBLE");
                        } else {
                            let violated = config.blame().len();
                            eprintln!("  Solution violates {} constraint(s)", violated);
                        }
                    }
                    Err(check) => {
                        eprintln!(
                            "  Config build failed: missing={}, excess={}, non_conforming={}",
                            check.missing_variables.len(),
                            check.excess_variables.len(),
                            check.non_conforming_variables.len(),
                        );
                    }
                }
            } else {
                eprintln!("  No solution returned");
            }

            if outcome.status == SolveStatus::Error {
                eprintln!("  Strategy reported an error");
            }
        }
        Err(_) => {
            eprintln!("  Channel closed without receiving a result");
            if let Some(StrategyProgressData::Incremental(p)) = handle.last_progress() {
                eprintln!("  Last progress: {p}");
            }
        }
    }
}

async fn conductor_solve(model: &collomatique_constraints_colloscopes::ColloscopeModel) {
    use collomatique_strategies::{
        ConductorPayload, ConductorProgress, ConductorStrategy, SolveStatus, Strategy,
        StrategyContext,
    };
    use collomatique_subprocesses::SubprocessSolveBackend;
    use std::sync::Arc;

    type V = collomatique_ilp_modeler::InternalVar<
        collomatique_constraints_colloscopes::Var,
        collomatique_constraints_colloscopes::ExtraVarName,
    >;

    let t = Instant::now();
    eprintln!("Extracting problem descriptor...");
    let (model_desc, _) = model.to_desc();
    eprintln!(
        "  Descriptor: {} variables, {} constraints ({:.2?})",
        model_desc.main.problem_desc.variables.len(),
        model_desc.main.problem_desc.constraints.len(),
        t.elapsed()
    );

    let backend = Arc::new(SubprocessSolveBackend::new());
    let on_echo: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|line: String| {
        eprint!("  [conductor] {}", line);
    });
    let ctx = StrategyContext::with_echo(backend, on_echo);

    let conductor = ConductorStrategy::default();

    eprintln!("Running conductor strategy...");
    let t = Instant::now();
    let result = conductor
        .run_with_callback(
            &ctx,
            model,
            None,
            ConductorPayload::default(),
            &|progress: ConductorProgress<V>| {
                match &progress {
                    ConductorProgress::Conductor(status) => {
                        let obj_str = status
                            .best_solution
                            .as_ref()
                            .map(|s| format!("{:.4}", s.objective))
                            .unwrap_or_else(|| "N/A".to_string());
                        let bound_str = status
                            .best_bound
                            .map(|b| format!("{:.4}", b))
                            .unwrap_or_else(|| "N/A".to_string());
                        eprintln!("  [conductor] obj={} bound={}", obj_str, bound_str,);
                    }
                    ConductorProgress::WorkerAssigned {
                        worker_num,
                        strategy,
                    } => match strategy {
                        Some(s) => {
                            eprintln!("  [conductor] worker {worker_num} assigned: {}", s.name())
                        }
                        None => eprintln!("  [conductor] worker {worker_num} idle"),
                    },
                    ConductorProgress::WorkerProgress {
                        worker_num,
                        progress,
                    } => {
                        eprintln!("  [conductor] worker {worker_num}: {progress}");
                    }
                    ConductorProgress::WorkerEcho { worker_num, echo } => {
                        eprint!("  [conductor] [worker {worker_num}] {}", echo);
                    }
                }
                true
            },
        )
        .await;

    match result {
        Ok(outcome) => {
            eprintln!("  Result received in {:.2?}", t.elapsed());
            eprintln!("  Status: {:?}", outcome.status);
            match outcome.objective {
                Some(v) => eprintln!("  Objective: {}", v),
                None => eprintln!("  Objective: N/A"),
            }
            match outcome.best_bound {
                Some(v) => eprintln!("  Best bound: {}", v),
                None => eprintln!("  Best bound: N/A"),
            }

            if let Some(ref config_data) = outcome.solution {
                let problem = model.problem();
                match problem.build_config(config_data.clone()) {
                    Ok(config) => {
                        if config.is_feasible() {
                            eprintln!("  Solution is FEASIBLE");
                        } else {
                            let violated = config.blame().len();
                            eprintln!("  Solution violates {} constraint(s)", violated);
                        }
                    }
                    Err(check) => {
                        eprintln!(
                            "  Config build failed: missing={}, excess={}, non_conforming={}",
                            check.missing_variables.len(),
                            check.excess_variables.len(),
                            check.non_conforming_variables.len(),
                        );
                    }
                }
            } else {
                eprintln!("  No solution returned");
            }

            if outcome.status == SolveStatus::Error {
                eprintln!("  Conductor reported an error");
            }
        }
        Err(e) => {
            eprintln!("  Conductor strategy failed in {:.2?}: {}", t.elapsed(), e);
        }
    }
}
