//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use anyhow::anyhow;
use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Args {
    /// Ignore all other parameters and run the python engine
    #[arg(long, default_value_t = false)]
    rpc_engine: bool,

    /// Open Collomatique directly editing a new colloscope
    #[arg(short, long, default_value_t = false)]
    new: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,

    /// Everything after gets passed through to GTK.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    gtk_options: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if args.rpc_engine {
        return run_rpc_engine();
    }

    let payload = collomatique_gtk4::AppInit {
        new: args.new,
        file_name: args.file,
    };

    let program_invocation = std::env::args().next().unwrap();
    let mut gtk_args = vec![program_invocation];
    gtk_args.extend(args.gtk_options.clone());

    let app = RelmApp::new("fr.collomatique.gtk4").with_args(gtk_args);
    app.allow_multiple_instances(true);
    app.run::<AppModel>(payload);

    Ok(())
}

/// Main RPC Engine function
///
/// Runs the RPC engine through stdin/stdout
pub fn run_rpc_engine() -> Result<(), anyhow::Error> {
    eprintln!("Waiting for initial payload...");
    let init_msg = match collomatique_rpc::wait_for_init_msg() {
        Ok(x) => x,
        Err(e) => return Err(anyhow!("Unknown initial payload: {}", e)),
    };
    eprintln!("Payload received!");

    match init_msg {
        collomatique_rpc::InitMsg::RpcTest => run_rpc_test()?,
        collomatique_rpc::InitMsg::RunPythonScript(script) => {
            collomatique_python::run_python_script(script)?
        }
    }

    eprintln!("Exiting...");
    collomatique_rpc::send_exit();

    Ok(())
}

fn run_rpc_test() -> Result<(), anyhow::Error> {
    eprintln!("Running RPC test...");
    for i in 1..=100 {
        eprintln!("Msg {}", i);
        match i % 3 {
            1 => match collomatique_rpc::send_rpc(collomatique_rpc::CmdMsg::Success) {
                Ok(rep) => {
                    if rep != collomatique_rpc::OutMsg::Ack {
                        return Err(anyhow!("Invalid response to valid command!"));
                    }
                }
                Err(e) => return Err(anyhow!("Invalid response: {}", e)),
            },
            2 => match collomatique_rpc::send_rpc(collomatique_rpc::CmdMsg::Warning) {
                Ok(rep) => {
                    if rep != collomatique_rpc::OutMsg::Ack {
                        return Err(anyhow!("Invalid response to valid command!"));
                    }
                }
                Err(e) => return Err(anyhow!("Invalid response: {}", e)),
            },
            _ => match collomatique_rpc::send_raw_msg("This is not a valid command...") {
                Ok(rep) => {
                    if rep != collomatique_rpc::OutMsg::Invalid {
                        return Err(anyhow!("Ack response to invalid command!"));
                    }
                }
                Err(e) => return Err(anyhow!("Invalid response: {}", e)),
            },
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    eprintln!("End of RPC test");
    Ok(())
}
