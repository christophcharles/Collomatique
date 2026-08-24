use anyhow::anyhow;
use collomatique_rpc::{
    InitMsg, ResultMsg, SerializedIlpProblem, SerializedStrategyRequest, SolverIncumbentInfo,
    SolverMsg, SolverProgressData, SolverResultData, SolverStatus, StrategyMsg, StrategyResultData,
    StrategyStatus, send_command,
};
use collomatique_rpc_colloscopes::{
    AppAnswerMsg, AppCmdMsg, AppInitMsg, ColloCmdMsg, ColloProtocol, InternalDataStream,
};

#[cfg(test)]
mod tests;

/// Rate limiter for the solver's progress reports.
///
/// Each report is a blocking RPC round trip up to the conductor, and a solve
/// whose tree search restarts fires tens of thousands of events — around 19600
/// in the epoch that motivated this. Everything downstream of a report (the
/// strategies' distance cutoff, the debug view) is fine at 10 Hz.
///
/// This throttles reporting only. The solver's own time limits are checked on
/// every event, inside the solver and above the closure this guards, so they
/// keep their full resolution.
struct ProgressThrottle {
    last_sent: Option<std::time::Instant>,
}

impl ProgressThrottle {
    const MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

    fn new() -> Self {
        ProgressThrottle { last_sent: None }
    }

    /// Whether to report the event happening at `now`.
    ///
    /// `fresh_incumbent` must be true exactly when the event brought an
    /// incumbent that has not been reported yet. A strategy acts on those, so
    /// they are never dropped, however fast they arrive.
    fn should_send(&mut self, now: std::time::Instant, fresh_incumbent: bool) -> bool {
        let due = self
            .last_sent
            .is_none_or(|sent| now.duration_since(sent) >= Self::MIN_INTERVAL);
        if !due && !fresh_incumbent {
            return false;
        }
        self.last_sent = Some(now);
        true
    }
}

pub fn wait_for_init_msg() -> Result<InitMsg<ColloProtocol>, String> {
    collomatique_rpc::receive_init::<ColloProtocol>().map_err(|e| e.to_string())
}

pub fn send_exit() {
    // The last thing this process does, so there is nothing left to abandon if
    // it fails; the host learns the same thing from the channel closing.
    if let Err(e) = collomatique_rpc::send_graceful_exit() {
        eprintln!("Erreur à l'envoi du message de sortie : {e}");
    }
}

fn solve_ilp(serialized: SerializedIlpProblem) -> Result<(), anyhow::Error> {
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp::solvers::{
        IncumbentTimeLimitSolverModel, ProgressBounds, ProgressIncumbentData,
        ProgressIncumbentInfo, ProgressStats, Solver, WarmSolver,
    };
    use collomatique_ilp::{DefaultRepr, ProblemBuilder};
    use ordered_float::OrderedFloat;

    let request = collomatique_rpc::IlpSolveRequest::from(serialized);

    eprintln!("Building problem from desc...");
    let problem = ProblemBuilder::<usize, (), DefaultRepr<usize>>::from_desc(request.problem_desc)
        .build()
        .map_err(|e| anyhow!("Failed to build problem from desc: {:?}", e))?;

    let solver = ColloCbcSolver::with_disable_logging(request.disable_logging);

    let num_vars = problem.get_variables().len();
    let var_indices: Vec<usize> = (0..num_vars).collect();

    let model = if let Some(ref warm_start) = request.warm_start {
        let hint = collomatique_ilp::solution_to_config_data(warm_start, &var_indices);
        solver.build_warm_model(&problem, &hint)
    } else {
        solver.build_model(&problem)
    };

    let mut last_best_bound = 0.0f64;
    let mut last_node_count = 0u64;

    let mut throttle = ProgressThrottle::new();
    // What we answer on an event we do not report: the parent's last word on
    // whether to carry on. Before the first round trip that is "carry on".
    let mut last_control = true;

    eprintln!("Solving...");
    // Both time limits are enforced by the solver itself; the callback only reports
    // progress and relays the parent's stop request.
    let result = model.solve_with_time_limits(
        request.time_limit,
        request.incumbent_time_limit,
        |progress| {
            // These two feed the final result, so they track every event
            // whether or not it is reported upstream.
            last_best_bound = progress.best_bound();
            last_node_count = progress.nodes();

            if !throttle.should_send(std::time::Instant::now(), progress.incumbent_is_fresh()) {
                return last_control;
            }

            let progress_data = SolverProgressData {
                best_obj: progress.best_objective().map(OrderedFloat),
                best_bound: OrderedFloat(progress.best_bound()),
                node_count: progress.nodes(),
                solutions_found: progress.solutions(),
                incumbent_info: progress.incumbent_info().map(|info| SolverIncumbentInfo {
                    objective: OrderedFloat(info.objective),
                    feasible: info.feasible,
                }),
                incumbent_solution: progress.incumbent_data().map(|cfg| {
                    var_indices
                        .iter()
                        .map(|&i| OrderedFloat(cfg.get(i).unwrap_or(0.0)))
                        .collect()
                }),
            };

            let response = send_command(ColloCmdMsg::Solver(SolverMsg::Progress(progress_data)));
            last_control = matches!(response, Ok(ResultMsg::SolverControl(true)));
            last_control
        },
    );

    let status = match result.stopped {
        Some(reason) => SolverStatus::Stopped(reason),
        None if result.config.is_some() => SolverStatus::Optimal,
        None => SolverStatus::Infeasible,
    };

    let obj_value = result.config.as_ref().map(|c| OrderedFloat(c.eval()));

    let best_bound = if last_node_count > 0 || obj_value.is_some() {
        Some(OrderedFloat(last_best_bound))
    } else {
        None
    };

    let solution = result.config.map(|config| {
        var_indices
            .iter()
            .map(|&i| OrderedFloat(config.get(i).unwrap_or(0.0)))
            .collect::<Vec<_>>()
    });

    let result_data = SolverResultData {
        status,
        obj_value,
        best_bound,
        node_count: last_node_count,
        solution,
    };

    eprintln!("Sending result...");
    send_command(ColloCmdMsg::Solver(SolverMsg::Result(result_data)))
        .map_err(|e| anyhow!("Error sending SolverResult: {}", e))?;

    Ok(())
}

fn run_strategy(serialized: SerializedStrategyRequest) -> Result<(), anyhow::Error> {
    use collomatique_ilp_modeler::InternalVar;
    use collomatique_rpc::{SerializedStrategyProgress, StrategyProgressRaw};
    use collomatique_strategies::{
        Strategy, StrategyContext, StrategyPayload, StrategyProgress, StrategyRequest,
        VarOrderSerializable,
    };
    use collomatique_subprocesses::{EngineExe, SubprocessSolveBackend};
    use ordered_float::OrderedFloat;
    use std::sync::Arc;

    let request_str: String = serialized.into();
    let request = StrategyRequest::deserialize(&request_str)
        .map_err(|e| anyhow!("Failed to deserialize strategy request: {e}"))?;

    eprintln!("Building model from desc...");
    let (model, var_order) = request.model_desc.to_model();

    let warm_start = request
        .warm_start
        .as_ref()
        .map(|raw| collomatique_ilp::solution_to_config_data(raw, &var_order));

    // Reconstruct the typed payload from its erased form against this subprocess's var_order,
    // mirroring how progress is erased on the way back out.
    let payload = <StrategyPayload<usize, usize> as VarOrderSerializable<
        InternalVar<usize, usize>,
    >>::from_data(&request.payload, &var_order)
    .unwrap_or_else(|e| match e {});

    // Nested workers spawned from inside the engine process: `Current` is right even when
    // this engine was itself launched by explicit path, since `current_exe()` here is the
    // very binary that was named.
    //
    // Each one binds its own listener and sets COLLOMATIQUE_RPC_CHANNEL for the child it
    // spawns, overriding the value this process inherited. So a nested worker talks to this
    // engine, not past it to the GUI.
    let backend = Arc::new(SubprocessSolveBackend::new(EngineExe::Current));
    let strategy_name = request.strategy.name();
    let on_echo: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |line: String| {
        eprint!("[{strategy_name} strategy] {}", line);
    });
    let ctx = StrategyContext::with_echo(backend, on_echo);

    let progress_callback = |progress: StrategyProgress<InternalVar<usize, usize>>| -> bool {
        // Erase the typed progress to its serializable form for the IPC barrier.
        let data =
            VarOrderSerializable::into_data(&progress, &var_order).unwrap_or_else(|e| match e {});
        eprintln!("[{strategy_name} strategy progress] {data}");
        let serialized_progress = data.serialize();
        let progress_raw = StrategyProgressRaw {
            progress: SerializedStrategyProgress::from(serialized_progress),
        };
        let response = send_command(ColloCmdMsg::Strategy(StrategyMsg::Progress(progress_raw)));
        match response {
            Ok(ResultMsg::StrategyControl(cont)) => cont,
            _ => false,
        }
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    eprintln!("Running strategy...");
    let outcome = rt
        .block_on(request.strategy.run_with_callback(
            &ctx,
            &model,
            warm_start,
            payload,
            &progress_callback,
        ))
        .map_err(|e| anyhow!("Strategy failed: {e}"))?;

    let status = match outcome.status {
        collomatique_strategies::SolveStatus::Optimal => StrategyStatus::Optimal,
        collomatique_strategies::SolveStatus::Infeasible => StrategyStatus::Infeasible,
        collomatique_strategies::SolveStatus::Stopped(reason) => StrategyStatus::Stopped(reason),
        collomatique_strategies::SolveStatus::Error => StrategyStatus::Error,
    };

    let solution = outcome.solution.map(|config| {
        var_order
            .iter()
            .map(|iv: &InternalVar<usize, usize>| {
                OrderedFloat(config.get(iv.clone()).unwrap_or(0.0))
            })
            .collect::<Vec<_>>()
    });

    let result_data = StrategyResultData {
        status,
        objective: outcome.objective.map(OrderedFloat),
        best_bound: outcome.best_bound.map(OrderedFloat),
        solution,
    };

    eprintln!("Sending strategy result...");
    send_command(ColloCmdMsg::Strategy(StrategyMsg::Result(result_data)))
        .map_err(|e| anyhow!("Error sending StrategyResult: {e}"))?;

    Ok(())
}

/// The application, as a hosted python script sees it
///
/// The script edits a copy and hands one back when it decides to; there is no
/// stream of edits and no merge, so this is the whole document in either
/// direction. The GUI already accepts `SetData` at any moment and any number of
/// times, applying each one onto the session it commits at the end of the run,
/// so a send needs nothing new on the other side of the pipe.
struct RpcHost {
    /// What the engine fetched before starting the script
    ///
    /// Held rather than re-fetched: the document the script is offered is the
    /// one the GUI had when the run started, and a second `GetData` mid-run
    /// would be a different document.
    data: collomatique_state_colloscopes::Data,
}

impl collomatique_python_runner::Host for RpcHost {
    fn data(&self) -> collomatique_state_colloscopes::Data {
        self.data.clone()
    }

    fn send(&self, data: &collomatique_state_colloscopes::Data) -> Result<(), String> {
        let data_stream = InternalDataStream::from(data);
        send_command(ColloCmdMsg::App(AppCmdMsg::SetData(data_stream)))
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Main RPC Engine function
///
/// Runs the RPC engine on the channel named in the environment by whoever spawned it.
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    // Insurance for the Unix subprocess-teardown mechanism: children die when their
    // parent dies because closing the parent-held pty master hangs up the child's
    // controlling terminal, delivering SIGHUP (default disposition: terminate). Reset
    // SIGHUP to SIG_DFL up front so that a signal handler installed by a linked library
    // (e.g. a future CBC build) can never leave a worker alive through the hangup.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_DFL);
    }

    // Before anything else: the protocol lives on its own channel, and nothing
    // below can say a word until this process has joined it. stdout and stderr
    // stay what they look like — output for whoever is watching.
    collomatique_rpc::connect_channel()
        .map_err(|e| anyhow!("Impossible de rejoindre le canal RPC : {e}"))?;

    eprintln!("Waiting for initial payload...");
    let init_msg = match wait_for_init_msg() {
        Ok(x) => x,
        Err(e) => return Err(anyhow!("Unknown initial payload: {}", e)),
    };
    eprintln!("Payload received!");

    match init_msg {
        InitMsg::App(AppInitMsg::RunPythonScript(script)) => {
            eprintln!("Receiving file data...");
            let data_msg = send_command(ColloCmdMsg::App(AppCmdMsg::GetData))
                .map_err(|e| anyhow!("Error on GetData: {}", e))?;
            let data = match data_msg {
                ResultMsg::App(AppAnswerMsg::Data(data)) => {
                    collomatique_state_colloscopes::Data::from(data)
                }
                _ => return Err(anyhow!("Bad Data packet: {:?}", data_msg)),
            };
            let host = std::sync::Arc::new(RpcHost { data });

            eprintln!("Running Python script...");
            collomatique_python_runner::initialize();
            // A hosted process is a collomatique binary, so the running
            // executable is an engine a script's solve may re-execute —
            // hosted or not, since a script may solve a document it loaded
            // itself.
            collomatique_python_runner::run_python_script(
                script,
                Some(host),
                Some(collomatique_python_runner::EngineExe::Current),
            )?;

            // Nothing is sent back here: a script sends when it says so
            // (`docs/python/new_api_design.md` §9.2), through the `send` of
            // `RpcHost` above.
        }
        InitMsg::SolveIlp(serialized) => {
            solve_ilp(serialized)?;
        }
        InitMsg::RunStrategy(serialized) => {
            run_strategy(serialized)?;
        }
    }

    eprintln!("Exiting...");
    send_exit();

    Ok(())
}
