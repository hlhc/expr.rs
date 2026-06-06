//!
//! Example:
//! ```
//! use expr::{Context, Parser};
//!
//! let mut p = Parser::new();
//!
//! let mut ctx = Context::default();
//! ctx.insert("two".to_string(), 2);
//!
//! let three: i64 = p.eval("1 + two", &ctx).unwrap().as_number().unwrap();
//! assert_eq!(three, 3);
//!
//! p.add_function("add", |c| {
//!   let mut sum = 0;
//!   for arg in c.args {
//!     sum += arg.as_number().unwrap();
//!   }
//!   Ok(sum.into())
//! });
//!
//! let six: i64 = p.eval("add(1, two, 3)", &ctx).unwrap().as_number().unwrap();
//! assert_eq!(six, 6);
//! ```
mod ast;
mod context;
mod error;
mod eval;
mod functions;
mod optimizers;
mod parser;
mod pest;
mod pratt;
#[cfg(feature = "serde")]
mod serde;
#[cfg(test)]
mod test;
mod value;

pub use crate::ast::node::Node;
pub use crate::ast::program::Program;
pub use crate::context::Context;
pub use crate::error::{Error, Result};
pub use crate::eval::{CompileOpts, Environment, eval, run};
#[allow(deprecated)]
pub use crate::parser::Parser;
#[allow(deprecated)]
pub use crate::parser::compile;
#[cfg(feature = "serde")]
pub use crate::serde::{from_value, to_value};
pub use crate::value::{FloatKey, MapKey, Value};

use pest_derive::Parser as PestParser;

#[derive(PestParser)]
#[grammar = "expr.pest"]
pub(crate) struct ExprPest;

#[macro_use]
mod macros;

// Non-public API. Used from macro-generated code.
#[doc(hidden)]
pub mod __private {
    #[doc(hidden)]
    pub use indexmap::IndexMap;
}
