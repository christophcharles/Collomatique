use anyhow::anyhow;
use collomatique_python_runner::{SendError, TakenDocument};
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
    fn live(&self) -> bool {
        false
    }

    fn data(&self) -> Result<TakenDocument, String> {
        Ok(TakenDocument {
            data: self.data.clone(),
            token: None,
        })
    }

    fn send(
        &self,
        data: &collomatique_state_colloscopes::Data,
        _token: Option<u64>,
    ) -> Result<Option<u64>, SendError> {
        let data_stream = InternalDataStream::from(data);
        send_command(ColloCmdMsg::App(AppCmdMsg::SetData(data_stream)))
            .map(|_| None)
            .map_err(|e| SendError::Failed(e.to_string()))
    }
}

/// The application, as the interactive console sees it
///
/// Unlike [`RpcHost`], it asks again every time: the user goes on editing while
/// the console is open, so the document it hands over is the one the
/// application holds at that moment, named by a token the application
/// recognises later.
struct ReplHost;

impl collomatique_python_runner::Host for ReplHost {
    fn live(&self) -> bool {
        true
    }

    fn data(&self) -> Result<TakenDocument, String> {
        let answer =
            send_command(ColloCmdMsg::App(AppCmdMsg::GetData)).map_err(|e| e.to_string())?;
        match answer {
            ResultMsg::App(AppAnswerMsg::Data(data_stream)) => {
                let token = data_stream.token();
                Ok(TakenDocument {
                    data: collomatique_state_colloscopes::Data::from(data_stream),
                    token: Some(token),
                })
            }
            other => Err(format!("Bad Data packet: {other:?}")),
        }
    }

    fn send(
        &self,
        data: &collomatique_state_colloscopes::Data,
        token: Option<u64>,
    ) -> Result<Option<u64>, SendError> {
        let data_stream = InternalDataStream::from(data);
        match send_command(ColloCmdMsg::App(AppCmdMsg::ReplaceData {
            data: data_stream,
            token,
        })) {
            Ok(ResultMsg::App(AppAnswerMsg::ReplaceDone { token })) => Ok(Some(token)),
            Ok(ResultMsg::App(AppAnswerMsg::ReplaceRefused)) => Err(SendError::Refused(
                String::from("l'utilisateur a refusé de remplacer le document ouvert"),
            )),
            Ok(ResultMsg::GlobalError(e)) => Err(SendError::Failed(e)),
            Ok(other) => Err(SendError::Failed(format!(
                "Bad ReplaceData answer: {other:?}"
            ))),
            Err(e) => Err(SendError::Failed(e.to_string())),
        }
    }
}

/// The console's keyboard, one line per round trip
struct RpcReplIo;

impl collomatique_python_runner::ReplIo for RpcReplIo {
    fn read_line(&self, prompt: &str) -> Result<String, String> {
        match send_command(ColloCmdMsg::App(AppCmdMsg::ReadLine {
            prompt: prompt.to_owned(),
        })) {
            Ok(ResultMsg::App(AppAnswerMsg::Line(line))) => Ok(line),
            Ok(other) => Err(format!("Bad ReadLine answer: {other:?}")),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn run_python_repl() -> Result<(), anyhow::Error> {
    eprintln!("Starting Python console...");
    collomatique_python_runner::initialize();
    collomatique_python_runner::run_python_repl(
        Some(std::sync::Arc::new(ReplHost)),
        Some(collomatique_python_runner::EngineExe::Current),
        std::sync::Arc::new(RpcReplIo),
    )
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

    // Nothing is sent back here: a script sends when it says so, through the
    // `send` of `RpcHost` above.
    Ok(())
}

/// Main RPC Engine function, for the colloscope application
///
/// The generic engine answers the ILP and strategy jobs on its own; this half
/// adds the hosted python script and the interactive console, the two jobs that
/// need a colloscope document.
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    collomatique_rpc_engine::run_engine::<ColloProtocol, _>(|app| match app {
        AppInitMsg::RunPythonScript(script) => run_python_script(script),
        AppInitMsg::StartPythonRepl => run_python_repl(),
    })
}
