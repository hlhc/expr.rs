use crate::Rule;
use crate::ast::node::Node;
use crate::functions::FuncArgs;
use crate::{Context, Environment, Value};
use crate::{MapKey, Result, bail};
use log::trace;
use pest::iterators::Pair;
use std::collections::HashSet;
use std::iter::once;

#[derive(Debug, Clone, PartialEq, strum::Display)]
pub enum PostfixOperator {
    Index { idx: Box<Node>, optional: bool },
    Range(Option<i64>, Option<i64>),
    Default(Box<Node>),
    Pipe(Box<Node>),
    Ternary { left: Box<Node>, right: Box<Node> },
}

impl PostfixOperator {
    pub(crate) fn contains_hash_ident(&self) -> bool {
        match self {
            PostfixOperator::Index { idx, .. } => idx.contains_hash_ident(),
            PostfixOperator::Default(node) | PostfixOperator::Pipe(node) => {
                node.contains_hash_ident()
            }
            PostfixOperator::Ternary { left, right } => {
                left.contains_hash_ident() || right.contains_hash_ident()
            }
            PostfixOperator::Range(..) => false,
        }
    }

    pub fn contains_func_call(&self) -> bool {
        match self {
            PostfixOperator::Index { idx, .. } => idx.contains_func_call(),
            PostfixOperator::Default(node) => node.contains_func_call(),
            PostfixOperator::Pipe(_) => true,
            PostfixOperator::Ternary { left, right } => {
                left.contains_func_call() || right.contains_func_call()
            }
            PostfixOperator::Range(..) => false,
        }
    }

    pub fn collect_idents(&self, set: &mut HashSet<String>) {
        match self {
            PostfixOperator::Index { idx, .. } => idx.collect_idents(set),
            PostfixOperator::Default(node) => node.collect_idents(set),
            PostfixOperator::Pipe(func) => func.collect_idents(set),
            PostfixOperator::Ternary { left, right } => {
                left.collect_idents(set);
                right.collect_idents(set);
            }
            PostfixOperator::Range(..) => {}
        }
    }
}

impl From<Pair<'_, Rule>> for PostfixOperator {
    fn from(pair: Pair<Rule>) -> Self {
        trace!("{:?}={}", pair.as_rule(), pair.as_str());
        match pair.as_rule() {
            Rule::index_op | Rule::member_op => PostfixOperator::Index {
                idx: Box::new(pair.into_inner().into()),
                optional: false,
            },
            Rule::optional_index_op | Rule::optional_member_op => PostfixOperator::Index {
                idx: Box::new(pair.into_inner().into()),
                optional: true,
            },
            Rule::slice_from_op => {
                let mut inner = pair.into_inner();
                let start = inner.next().map(|p| p.as_str().parse().unwrap());
                let end = inner.next().map(|p| p.as_str().parse().unwrap());
                PostfixOperator::Range(start, end)
            }
            Rule::slice_to_op => {
                let mut inner = pair.into_inner();
                let end = inner.next().map(|p| p.as_str().parse().unwrap());
                PostfixOperator::Range(None, end)
            }
            Rule::default_op => PostfixOperator::Default(Box::new(pair.into_inner().into())),
            Rule::ternary_op => {
                let mut inner = pair.into_inner();
                let left = Box::new(inner.next().unwrap().into());
                let right = Box::new(inner.next().unwrap().into());
                PostfixOperator::Ternary { left, right }
            }
            Rule::pipe_op => PostfixOperator::Pipe(Box::new(pair.into_inner().into())),
            rule => unreachable!("Unexpected rule: {rule:?}"),
        }
    }
}

impl Environment<'_> {
    pub fn eval_postfix_operator(
        &self,
        ctx: &Context,
        operator: PostfixOperator,
        node: Node,
    ) -> Result<Value> {
        let value = self.eval_expr(ctx, node)?;
        let result = match operator {
            PostfixOperator::Index { idx, optional } => match self.eval_index_key(ctx, *idx)? {
                Value::Number(idx) => match value {
                    Value::Array(arr) => {
                        let idx = i64_to_idx(idx, arr.len());
                        arr.get(idx).cloned().unwrap_or(Value::Nil)
                    }
                    Value::Map(map) => map.get(&MapKey::Number(idx)).cloned().unwrap_or(Value::Nil),
                    _ if optional => Value::Nil,
                    _ => bail!("Invalid operand for operator []"),
                },
                Value::String(key) => match value {
                    Value::Map(map) => map.get(&MapKey::String(key)).cloned().unwrap_or(Value::Nil),
                    _ if optional => Value::Nil,
                    _ => bail!("Invalid operand for operator []"),
                },
                v => bail!("Invalid operand for operator []: {v:?}"),
            },
            PostfixOperator::Range(start, end) => match value {
                Value::Array(arr) => {
                    let start = i64_to_idx(start.unwrap_or(0), arr.len());
                    let end = i64_to_idx(end.unwrap_or(arr.len() as i64), arr.len());
                    let result = arr[start..end].to_vec();
                    Value::Array(result)
                }
                _ => bail!("Invalid operand for operator []"),
            },
            PostfixOperator::Default(default) => match value {
                Value::Nil => self.eval_expr(ctx, *default)?,
                value => value,
            },
            PostfixOperator::Ternary { left, right } => match value {
                Value::Bool(true) => self.eval_expr(ctx, *left)?,
                Value::Bool(false) => self.eval_expr(ctx, *right)?,
                value => bail!("Invalid condition for ?: {value:?}"),
            },
            PostfixOperator::Pipe(func) => {
                if let Node::Func {
                    ident,
                    args,
                    predicate,
                    threshold,
                    throws,
                    map_node,
                } = *func
                {
                    let args = args
                        .into_iter()
                        .map(|arg| self.eval_expr(ctx, arg))
                        .chain(once(Ok(value)))
                        .collect::<Result<Vec<Value>>>()?;
                    self.eval_func(
                        ctx,
                        FuncArgs {
                            ident,
                            args,
                            predicate: predicate.map(|p| *p),
                            threshold,
                            throws,
                            map_node,
                        },
                    )?
                } else {
                    bail!("Invalid operand for operator |");
                }
            }
        };

        Ok(result)
    }

    fn eval_index_key(&self, ctx: &Context, idx: Node) -> Result<Value> {
        match idx {
            Node::Value(v) => Ok(v),
            Node::Ident(id) => Ok(Value::String(id)),
            idx => self.eval_expr(ctx, idx),
        }
    }
}

fn i64_to_idx(idx: i64, len: usize) -> usize {
    if idx < 0 {
        (len as i64 + idx) as usize
    } else {
        idx as usize
    }
}
