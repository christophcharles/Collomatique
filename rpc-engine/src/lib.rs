use anyhow::anyhow;
use collomatique_rpc::InternalDataStream;
use collomatique_rpc::{
    CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg, SerializedIlpProblem,
    SolverIncumbentInfo, SolverMsg, SolverProgressData, SolverResultData, SolverStatus,
};

pub fn wait_for_init_msg() -> Result<InitMsg, String> {
    let encoded_msg = EncodedMsg::receive()?;
    encoded_msg.try_into()
}

pub fn send_exit() {
    let msg = CompleteCmdMsg::GracefulExit;
    let encoded_msg = EncodedMsg::from(msg);
    encoded_msg.send();
}

async fn try_solve() -> Result<(), anyhow::Error> {
    use anyhow::anyhow;
    use std::time::Instant;

    let data_msg =
        EncodedMsg::send_rpc(CmdMsg::GetData).map_err(|e| anyhow!("Error on GetData: {}", e))?;
    let inner_data = match data_msg {
        ResultMsg::Data(data) => collomatique_state_colloscopes::InnerData::from(data),
        _ => return Err(anyhow!("Bad Data packet: {:?}", data_msg)),
    };

    let t_build = Instant::now();
    eprintln!("Building ILP problem...");

    let pool = sqlx::SqlitePool::connect(":memory:")
        .await
        .map_err(|e| anyhow!("Error connecting to in-memory DB: {}", e))?;
    collomatique_sqlite_state::create_schema(&pool)
        .await
        .map_err(|e| anyhow!("Error creating schema: {}", e))?;
    collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data)
        .await
        .map_err(|e| anyhow!("Error populating DB: {}", e))?;

    let export_config = inner_data.export_config;
    let env = inner_data.params;
    let problem = collomatique_constraints_colloscopes::build_model_with_log(&pool, &mut |msg| {
        eprintln!("  {msg}")
    })
    .await;
    eprintln!("ILP problem built in {:.2?}", t_build.elapsed());
    let stats = problem.stats();
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

    println!("Solving ILP problem...");
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(false);
    let sol_opt = problem.solve(&solver);
    let Some(sol) = sol_opt else {
        println!("No solution found");
        return Ok(());
    };
    println!("Solution found!");
    let config_data = sol.get_data();
    let new_colloscope =
        collomatique_constraints_colloscopes::convert::build_colloscope(&env, &config_data)
            .expect("Config data should be compatible with colloscope parameters");

    println!("Sending updated data...");
    let new_inner_data = collomatique_state_colloscopes::InnerData {
        params: env,
        colloscope: new_colloscope,
        export_config,
    };
    let data_stream = InternalDataStream::from(&new_inner_data);
    EncodedMsg::send_rpc(CmdMsg::SetData(data_stream))
        .map_err(|e| anyhow!("Error on SetData: {}", e))?;

    println!("Done.");

    Ok(())
}

fn solve_ilp(serialized: SerializedIlpProblem) -> Result<(), anyhow::Error> {
    use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
    use collomatique_ilp::solvers::{
        CallbackSolverModel, ProgressBounds, ProgressIncumbentInfo, ProgressStats, Solver,
        WarmSolver,
    };
    use collomatique_ilp::{DefaultRepr, ProblemBuilder};
    use ordered_float::OrderedFloat;
    use std::time::Instant;

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

    let start = Instant::now();
    let time_limit = request
        .time_limit_seconds
        .map(|s| std::time::Duration::from_secs(s as u64));

    eprintln!("Solving...");
    let result = model.solve_with_callback(|progress| {
        let progress_data = SolverProgressData {
            best_obj: OrderedFloat(progress.best_objective()),
            best_bound: OrderedFloat(progress.best_bound()),
            node_count: progress.nodes(),
            solutions_found: progress.solutions(),
            incumbent_info: progress.incumbent_info().map(|info| SolverIncumbentInfo {
                objective: OrderedFloat(info.objective),
                feasible: info.feasible,
            }),
        };

        let response = EncodedMsg::send_rpc(CmdMsg::Solver(SolverMsg::Progress(progress_data)));
        let should_continue = match response {
            Ok(ResultMsg::SolverControl(cont)) => cont,
            _ => false,
        };

        let time_ok = time_limit
            .map(|limit| start.elapsed() < limit)
            .unwrap_or(true);

        should_continue && time_ok
    });

    let status = if result.config.is_some() {
        if result.stopped_by_callback {
            SolverStatus::Stopped
        } else {
            SolverStatus::Optimal
        }
    } else if result.stopped_by_callback {
        SolverStatus::Stopped
    } else {
        SolverStatus::Infeasible
    };

    let obj_value = result
        .config
        .as_ref()
        .map(|c| c.eval())
        .unwrap_or(f64::INFINITY);

    let solution = result.config.map(|config| {
        var_indices
            .iter()
            .map(|&i| OrderedFloat(config.get(i).unwrap_or(0.0)))
            .collect::<Vec<_>>()
    });

    let result_data = SolverResultData {
        status,
        obj_value: OrderedFloat(obj_value),
        best_bound: OrderedFloat(f64::NEG_INFINITY),
        node_count: 0,
        solution,
    };

    eprintln!("Sending result...");
    EncodedMsg::send_rpc(CmdMsg::Solver(SolverMsg::Result(result_data)))
        .map_err(|e| anyhow!("Error sending SolverResult: {}", e))?;

    Ok(())
}

/// Main RPC Engine function
///
/// Runs the RPC engine through stdin/stdout
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    eprintln!("Waiting for initial payload...");
    let init_msg = match wait_for_init_msg() {
        Ok(x) => x,
        Err(e) => return Err(anyhow!("Unknown initial payload: {}", e)),
    };
    eprintln!("Payload received!");

    match init_msg {
        InitMsg::RunPythonScript(script) => {
            eprintln!("Receiving file data...");
            let data_msg = EncodedMsg::send_rpc(CmdMsg::GetData)
                .map_err(|e| anyhow!("Error on GetData: {}", e))?;
            let inner_data = match data_msg {
                ResultMsg::Data(data) => collomatique_state_colloscopes::InnerData::from(data),
                _ => return Err(anyhow!("Bad Data packet: {:?}", data_msg)),
            };
            let data = collomatique_state_colloscopes::Data::from_inner_data(inner_data)
                .map_err(|e| anyhow!("Error building Data: {}", e))?;
            let app_state = collomatique_state::AppState::new(data);
            let shared = std::sync::Arc::new(std::sync::Mutex::new(app_state));

            eprintln!("Running Python script...");
            collomatique_python::initialize();
            collomatique_python::run_python_script(script, Some(shared.clone()))?;

            // Send back if modified
            {
                use collomatique_state::traits::Manager;
                let state = shared.lock().unwrap();
                if state.can_undo() {
                    eprintln!("Sending final file data...");
                    let inner_data = state.get_data().get_inner_data();
                    let data_stream = InternalDataStream::from(inner_data);
                    EncodedMsg::send_rpc(CmdMsg::SetData(data_stream))
                        .map_err(|e| anyhow!("Error on SetData: {}", e))?;
                }
            }
        }
        InitMsg::SolveColloscope => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(try_solve())?;
        }
        InitMsg::SolveIlp(serialized) => {
            solve_ilp(serialized)?;
        }
    }

    eprintln!("Exiting...");
    send_exit();

    Ok(())
}
