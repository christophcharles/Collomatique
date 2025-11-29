mod ast;
pub mod eval;
mod parser;
mod semantics;
pub mod traits;
pub use collo_ml_derive::{EvalObject, ViewObject};
pub use eval::{CheckedAST, ExprValue};
pub use semantics::ExprType;
pub use traits::{EvalObject, EvalVar, ViewBuilder, ViewObject};
