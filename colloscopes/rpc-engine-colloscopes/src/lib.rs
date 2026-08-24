use anyhow::anyhow;
use collomatique_rpc::{ResultMsg, send_command};
use collomatique_rpc_colloscopes::{
    AppAnswerMsg, AppCmdMsg, AppInitMsg, ColloCmdMsg, ColloProtocol, InternalDataStream,
};

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

fn run_python_script(script: String) -> Result<(), anyhow::Error> {
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
    // A hosted process is a collomatique binary, so the running executable is
    // an engine a script's solve may re-execute — hosted or not, since a script
    // may solve a document it loaded itself.
    collomatique_python_runner::run_python_script(
        script,
        Some(host),
        Some(collomatique_python_runner::EngineExe::Current),
    )?;

    // Nothing is sent back here: a script sends when it says so
    // (`docs/python/new_api_design.md` §9.2), through the `send` of `RpcHost`
    // above.
    Ok(())
}

/// Main RPC Engine function, for the colloscope application
///
/// The generic engine answers the ILP and strategy jobs on its own; the only
/// thing this half adds is the hosted python script, which is the one job that
/// needs a colloscope document.
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    collomatique_rpc_engine::run_engine::<ColloProtocol, _>(|app| match app {
        AppInitMsg::RunPythonScript(script) => run_python_script(script),
    })
}
