use std::fmt;
use std::fmt::{Debug, Formatter};

use indexmap::IndexMap;
use once_cell::sync::Lazy;
use pest::Parser as PestParser;

use crate::ast::node::Node;
use crate::ast::program::Program;
use crate::functions::{
    ExprCall, FuncArgs, FunctionDefinition, FunctionMetadata, array, json, string,
};
use crate::{Context, Error, ExprPest, Result, Rule, Value, bail};

/// Run a compiled expr program, using the default environment
pub fn run(program: Program, ctx: &Context) -> Result<Value> {
    DEFAULT_ENVIRONMENT.run(program, ctx)
}

/// Compile and run an expr program in one step, using the default environment.
///
/// Example:
/// ```
/// use expr::{Context, eval};
/// let ctx = Context::default();
/// assert_eq!(eval("1 + 2", &ctx).unwrap().to_string(), "3");
/// ```
pub fn eval(code: &str, ctx: &Context) -> Result<Value> {
    DEFAULT_ENVIRONMENT.eval(code, ctx)
}

/// Options for compiling an expr program.
pub struct CompileOpts {
    /// Whether to run the optimizer (constant folding, dead code elimination, etc.).
    /// Defaults to `true`.
    pub optimized: bool,
}

impl Default for CompileOpts {
    fn default() -> Self {
        CompileOpts { optimized: true }
    }
}

/// Struct containing custom environment setup for expr evaluation (e.g. custom
/// function definitions)
///
/// Example:
///
/// ```
/// use expr::{Context, Environment, Value};
/// let mut env = Environment::new();
/// let ctx = Context::default();
/// env.add_function("add", |c| {
///   let mut sum = 0;
///     for arg in c.args {
///       if let Value::Number(n) = arg {
///         sum += n;
///        } else {
///          panic!("Invalid argument: {arg:?}");
///        }
///     }
///   Ok(sum.into())
/// });
/// assert_eq!(env.eval("add(1, 2, 3)", &ctx).unwrap().to_string(), "6");
/// ```
pub struct Environment<'a> {
    pub(crate) functions: IndexMap<String, FunctionDefinition<'a>>,
}

impl Default for Environment<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Environment<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ExprEnvironment").finish()
    }
}

impl<'a> Environment<'a> {
    /// Create a new environment with default set of functions
    pub fn new() -> Self {
        let mut p = Self {
            functions: IndexMap::new(),
        };
        string::add_string_functions(&mut p);
        array::add_array_functions(&mut p);
        json::add_json_functions(&mut p);
        p
    }

    /// Add a function for expr programs to call
    ///
    /// Example:
    /// ```
    /// use expr::{Context, Environment, Value};
    /// let mut env = Environment::new();
    /// let ctx = Context::default();
    /// env.add_function("add", |c| {
    ///   let mut sum = 0;
    ///     for arg in c.args {
    ///       if let Value::Number(n) = arg {
    ///         sum += n;
    ///        } else {
    ///          panic!("Invalid argument: {arg:?}");
    ///        }
    ///     }
    ///   Ok(sum.into())
    /// });
    /// assert_eq!(env.eval("add(1, 2, 3)", &ctx).unwrap().to_string(), "6");
    /// ```
    pub fn add_function<F>(&mut self, name: &str, f: F)
    where
        F: Fn(ExprCall) -> Result<Value> + 'a + Sync + Send,
    {
        self.functions.insert(
            name.to_string(),
            FunctionDefinition {
                runtime: Box::new(f),
                metadata: FunctionMetadata { const_eval: None },
            },
        );
    }

    /// Register a built-in function with optimizer metadata (e.g. const-eval).
    /// Only used internally for the standard library.
    pub(crate) fn add_builtin_function<F>(&mut self, name: &str, f: F, metadata: FunctionMetadata)
    where
        F: Fn(ExprCall) -> Result<Value> + 'a + Sync + Send,
    {
        self.functions.insert(
            name.to_string(),
            FunctionDefinition {
                runtime: Box::new(f),
                metadata,
            },
        );
    }

    /// Run a compiled expr program
    pub fn run(&self, program: Program, ctx: &Context) -> Result<Value> {
        let mut ctx = ctx.clone();
        ctx.insert(
            "$env".to_string(),
            Value::Map(
                ctx.0
                    .iter()
                    .map(|(k, v)| (k.clone().into(), v.clone()))
                    .collect(),
            ),
        );
        for (id, expr) in program.lines {
            ctx.insert(id, self.eval_expr(&ctx, expr)?);
        }
        self.eval_expr(&ctx, program.expr)
    }

    /// Parse an expr program with environment-aware optimization.
    ///
    /// Unlike the free `compile()`, this version knows which built-in
    /// functions are still pure (have not been replaced by the user) and
    /// can constant-fold them at compile time.
    pub fn compile(&self, code: &str) -> Result<Program> {
        self.compile_opts(code, &CompileOpts::default())
    }

    /// Parse an expr program with explicit control over optimization.
    ///
    /// Set `opts.optimized` to `false` to skip the compile-time optimizer
    /// (useful for benchmarking or comparing AST sizes).
    pub fn compile_opts(&self, code: &str, opts: &CompileOpts) -> Result<Program> {
        #[cfg(debug_assertions)]
        pest::set_error_detail(true);
        let pairs = ExprPest::parse(Rule::full, code).map_err(|e| Error::PestError(Box::new(e)))?;
        let mut program: Program = pairs.into();
        if opts.optimized {
            program.optimize(Some(&self.functions));
        }
        Ok(program)
    }

    /// Compile and run an expr program in one step
    ///
    /// Example:
    /// ```
    /// use std::collections::HashMap;
    /// use expr::{Context, Environment};
    /// let env = Environment::new();
    /// let ctx = Context::default();
    /// assert_eq!(env.eval("1 + 2", &ctx).unwrap().to_string(), "3");
    /// ```
    pub fn eval(&self, code: &str, ctx: &Context) -> Result<Value> {
        let program = self.compile(code)?;
        self.run(program, ctx)
    }

    pub fn eval_expr(&self, ctx: &Context, node: Node) -> Result<Value> {
        let value = match node {
            Node::Value(value) => value,
            Node::Ident(id) => {
                if let Some(value) = ctx.get(&id) {
                    value.clone()
                } else if let Some(item) = ctx
                    .get("#")
                    .and_then(|o| o.as_map())
                    .and_then(|m| m.get(&crate::MapKey::String(id.clone())))
                {
                    item.clone()
                } else {
                    bail!("unknown variable: {id}")
                }
            }
            Node::Func {
                ident,
                args,
                predicate,
                threshold,
                throws,
                map_node,
            } => {
                let args = args
                    .into_iter()
                    .map(|e| self.eval_expr(ctx, e))
                    .collect::<Result<_>>()?;
                self.eval_func(
                    ctx,
                    FuncArgs {
                        ident,
                        args,
                        predicate: predicate.map(|l| *l),
                        threshold,
                        throws,
                        map_node,
                    },
                )?
            }
            Node::Operation {
                left,
                operator,
                right,
            } => self.eval_operator(ctx, operator, *left, *right)?,
            Node::Unary { operator, node } => self.eval_unary_operator(ctx, operator, *node)?,
            Node::Postfix { operator, node } => self.eval_postfix_operator(ctx, operator, *node)?,
            Node::Array(a) => Value::Array(
                a.into_iter()
                    .map(|e| self.eval_expr(ctx, e))
                    .collect::<Result<_>>()?,
            ), // node => bail!("unexpected node: {node:?}"),
            Node::Range(start, end) => {
                match (self.eval_expr(ctx, *start)?, self.eval_expr(ctx, *end)?) {
                    (Value::Number(start), Value::Number(end)) => {
                        Value::Array((start..=end).map(Value::Number).collect())
                    }
                    (start, end) => bail!("invalid range: {start:?}..{end:?}"),
                }
            }
        };
        Ok(value)
    }
}

pub(crate) static DEFAULT_ENVIRONMENT: Lazy<Environment> = Lazy::new(|| Environment::new());
