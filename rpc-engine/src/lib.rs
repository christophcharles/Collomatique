use anyhow::anyhow;
use collomatique_rpc::InternalDataStream;
use collomatique_rpc::{CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg};

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

    let data_msg =
        EncodedMsg::send_rpc(CmdMsg::GetData).map_err(|e| anyhow!("Error on GetData: {}", e))?;
    let inner_data = match data_msg {
        ResultMsg::Data(data) => collomatique_state_colloscopes::InnerData::from(data),
        _ => return Err(anyhow!("Bad Data packet: {:?}", data_msg)),
    };
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
    let problem = collomatique_constraints_colloscopes::build_model(&pool).await;

    println!("Solving ILP problem...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
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
    }

    eprintln!("Exiting...");
    send_exit();

    Ok(())
}
