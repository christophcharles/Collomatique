//! Shared building blocks for turning the current document into an ILP model: the dialog that
//! configures what gets rebuilt ([`config_dialog`]) and the dialog that runs the build off-thread
//! ([`loading_dialog`]).

pub mod config_dialog;
pub mod loading_dialog;
