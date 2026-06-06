pub mod array;
pub mod json;
pub mod string;

use crate::Result;

use crate::ast::node::Node;
use crate::ast::program::Program;
use crate::{Context, Environment, Value, bail};

/// Compile-time constant evaluator for a built-in function.
/// Returns `Some(Value)` if all args are constant and the function
/// can be evaluated without side effects; `None` otherwise.
pub(crate) type ConstEvaluator = fn(&[Value]) -> Option<Value>;

/// Metadata attached to a function definition, used by the optimizer.
pub(crate) struct FunctionMetadata {
    /// If `Some`, this function is pure and can be evaluated at compile
    /// time when all arguments are constant.
    pub const_eval: Option<ConstEvaluator>,
}

/// A complete function definition: the runtime closure plus optimizer metadata.
pub(crate) struct FunctionDefinition<'a> {
    pub runtime: Box<dyn Fn(ExprCall) -> Result<Value> + 'a + Sync + Send>,
    pub metadata: FunctionMetadata,
}

/// Arguments passed to a function call at runtime.
pub(crate) struct FuncArgs {
    pub ident: String,
    pub args: Vec<Value>,
    pub predicate: Option<Program>,
    pub threshold: Option<i64>,
    pub throws: bool,
    pub map_node: Option<Box<Node>>,
}

pub struct ExprCall<'a, 'b> {
    pub ident: String,
    pub args: Vec<Value>,
    pub predicate: Option<Program>,
    pub ctx: &'a Context,
    pub env: &'a Environment<'b>,
    pub threshold: Option<i64>,
    pub throws: bool,
    pub map_node: Option<Box<Node>>,
}

impl Environment<'_> {
    pub(crate) fn eval_func(&self, ctx: &Context, call: FuncArgs) -> Result<Value> {
        let expr_call = ExprCall {
            ident: call.ident,
            args: call.args,
            predicate: call.predicate,
            ctx,
            env: self,
            threshold: call.threshold,
            throws: call.throws,
            map_node: call.map_node,
        };
        if let Some(def) = self.functions.get(&expr_call.ident) {
            (def.runtime)(expr_call)
        } else {
            bail!("Unknown function: {}", expr_call.ident)
        }
    }
}
