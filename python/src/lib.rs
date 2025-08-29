//! Collomatique-python crate
//!
//! This crate contains the code to run python code
//! as well as the necessary RCP code.

/// Main Python Engine function
///
/// Runs the Python engine through stdin/stderr
pub fn run_python_engine() {
    for i in 1..=100 {
        println!("Hello World! {}", i);
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}
