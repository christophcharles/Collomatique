use anyhow::anyhow;
use collomatique_rpc::{CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg};
use collomatique_state_colloscopes::ColloscopeOp;

pub fn wait_for_init_msg() -> Result<InitMsg, String> {
    let encoded_msg = EncodedMsg::receive()?;
    Ok(encoded_msg.try_into()?)
}

pub fn send_exit() {
    let msg = CompleteCmdMsg::GracefulExit;
    let encoded_msg = EncodedMsg::from(msg);
    encoded_msg.send();
}

fn try_solve() -> Result<(), anyhow::Error> {
    use anyhow::anyhow;

    let data_msg =
        EncodedMsg::send_rpc(CmdMsg::GetData).map_err(|e| anyhow!("Error on GetData: {}", e))?;
    let inner_data = match data_msg {
        ResultMsg::Data(data) => collomatique_state_colloscopes::InnerData::from(data),
        _ => return Err(anyhow!("Bad Data packet: {:?}", data_msg)),
    };
    let data = collomatique_state_colloscopes::Data::from_inner_data(inner_data)?;

    eprintln!("Building ILP problem...");

    use collomatique_binding_colloscopes::scripts::build_default_problem;
    let env = collomatique_binding_colloscopes::views::Env::from(data);
    let problem = build_default_problem(&env);

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
        collomatique_binding_colloscopes::convert::build_colloscope(&env, &config_data)
            .expect("Config data should be compatible with colloscope parameters");

    println!("Sending update ops...");
    let update_ops = env
        .data
        .get_inner_data()
        .colloscope
        .update_ops(new_colloscope)
        .expect("New and old colloscopes should be compatible");

    for op in update_ops {
        let dressed_op = match op {
            ColloscopeOp::UpdateGroupList(group_list_id, group_list) => {
                collomatique_ops::ColloscopeUpdateOp::UpdateColloscopeGroupList(
                    group_list_id,
                    group_list,
                )
            }
            ColloscopeOp::UpdateInterrogation(period_id, slot_id, week, interrogation) => {
                collomatique_ops::ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                    period_id,
                    slot_id,
                    week,
                    interrogation,
                )
            }
        };
        EncodedMsg::send_rpc(CmdMsg::Update(collomatique_ops::UpdateOp::Colloscope(
            dressed_op,
        )))
        .map_err(|e| anyhow!("Error on UpdateOp: {}", e))?;
    }

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
            collomatique_python::initialize();
            collomatique_python::run_python_script(script)?;
        }
        InitMsg::SolveColloscope => {
            try_solve()?;
        }
    }

    eprintln!("Exiting...");
    send_exit();

    Ok(())
}
