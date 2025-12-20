use anyhow::anyhow;
use collomatique_rpc::{wait_for_msg, CompleteCmdMsg, InitMsg};
use std::io::Write;

pub fn wait_for_init_msg() -> Result<InitMsg, String> {
    let data = wait_for_msg();
    InitMsg::from_text_msg(&data)
}

pub fn send_exit() {
    print!("{}\r\n", CompleteCmdMsg::GracefulExit.into_text_msg());
    std::io::stdout().flush().expect("no error on flush");
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
            eprintln!("/!\\ Solver with RPC not implemented yet!");
        }
    }

    eprintln!("Exiting...");
    send_exit();

    Ok(())
}
