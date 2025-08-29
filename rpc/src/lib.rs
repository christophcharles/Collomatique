//! Collomatique-rpc crate
//!
//! This crate contains the code to run an rpc server
//! as well as the necessary RCP messages

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitMsg {
    Greetings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutMsg {
    Ack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdMsg {
    Success,
    Warning,
}

/// Main RPC Engine function
///
/// Runs the RPC engine through stdin/stderr
pub fn run_rpc_engine() {
    for i in 1..=100 {
        println!("Hello World! {}", i);
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}
