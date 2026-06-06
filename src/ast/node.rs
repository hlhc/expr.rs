use indexmap::IndexMap;
use std::collections::HashSet;
use crate::ast::operator::Operator;
use crate::ast::postfix_operator::PostfixOperator;
use crate::ast::unary_operator::UnaryOperator;
use crate::pratt::PRATT_PARSER;
use crate::{Rule, Value};
use log::trace;
use pest::iterators::{Pair, Pairs};
use crate::ast::program::Program;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Ident(String),
    Array(Vec<Node>),
    Range(Box<Node>, Box<Node>),
    Value(Value),
    Func {
        ident: String,
        args: Vec<Node>,
        predicate: Option<Box<Program>>,
        threshold: Option<i64>,
        throws: bool,
        map_node: Option<Box<Node>>,
    },
    Unary {
        operator: UnaryOperator,
        node: Box<Node>,
    },
    Operation {
        operator: Operator,
        left: Box<Node>,
        right: Box<Node>,
    },
    Postfix {
        operator: PostfixOperator,
        node: Box<Node>,
    },
}

impl Node {
    /// Check if this node or any of its children reference the `#` variable
    pub(crate) fn contains_hash_ident(&self) -> bool {
        match self {
            Node::Ident(id) => id == "#" || id == "#acc" || id == "#index",
            Node::Operation { left, right, .. } => {
                left.contains_hash_ident() || right.contains_hash_ident()
            }
            Node::Unary { node, .. } => node.contains_hash_ident(),
            Node::Postfix { node, operator } => {
                node.contains_hash_ident() || operator.contains_hash_ident()
            }
            Node::Func { args, .. } => {
                // Only check args, not predicate — `#` inside a predicate is bound
                // to that function's iteration, not free for outer promotion.
                args.iter().any(|a| a.contains_hash_ident())
            }
            Node::Array(items) => items.iter().any(|i| i.contains_hash_ident()),
            Node::Range(a, b) => a.contains_hash_ident() || b.contains_hash_ident(),
            Node::Value(_) => false,
        }
    }

    /// Functions that accept predicates (closures over `#`).
    /// Matches Go expr-lang's hardcoded predicates map in parser.go.
    fn accepts_predicate(name: &str) -> bool {
        matches!(
            name,
            "all" | "any" | "one" | "none" | "map" | "filter" | "find"
                | "findIndex" | "findLast" | "findLastIndex"
                | "count" | "sum" | "reduce" | "groupBy" | "sortBy"
        )
    }

    /// Returns true if the entire subtree consists only of literal values (no
    /// identifiers, no function calls, no ranges). Used by the constant folder.
    pub fn is_constant(&self) -> bool {
        match self {
            Node::Value(_) => true,
            Node::Array(items) => items.iter().all(|i| i.is_constant()),
            Node::Operation { left, right, .. } => left.is_constant() && right.is_constant(),
            Node::Unary { node, .. } => node.is_constant(),
            Node::Range(_, _) => true, // grammar guarantees value..value
            Node::Postfix { node, operator } => {
                node.is_constant()
                    && match operator {
                        PostfixOperator::Index { idx, .. } => idx.is_constant(),
                        _ => false,
                    }
            }
            Node::Ident(_) | Node::Func { .. } => false,
        }
    }

    /// Returns true if this subtree contains any function call (including
    /// through postfix pipes). Used to avoid DCE on bindings that might have
    /// side effects from user-defined functions.
    pub fn contains_func_call(&self) -> bool {
        match self {
            Node::Func { .. } => true,
            Node::Array(items) => items.iter().any(|i| i.contains_func_call()),
            Node::Operation { left, right, .. } => {
                left.contains_func_call() || right.contains_func_call()
            }
            Node::Unary { node, .. } => node.contains_func_call(),
            Node::Postfix { node, operator } => {
                node.contains_func_call() || operator.contains_func_call()
            }
            Node::Range(_, _) | Node::Value(_) | Node::Ident(_) => false,
        }
    }

    /// Collect all identifier names referenced in this subtree into the given set.
    pub fn collect_idents(&self, set: &mut HashSet<String>) {
        match self {
            Node::Ident(id) => {
                set.insert(id.clone());
            }
            Node::Array(items) => {
                for item in items {
                    item.collect_idents(set);
                }
            }
            Node::Range(start, end) => {
                start.collect_idents(set);
                end.collect_idents(set);
            }
            Node::Value(_) => {}
            Node::Func {
                args,
                predicate,
                map_node,
                ..
            } => {
                for arg in args {
                    arg.collect_idents(set);
                }
                if let Some(p) = predicate {
                    for (_, val) in &p.lines {
                        val.collect_idents(set);
                    }
                    p.expr.collect_idents(set);
                }
                if let Some(mn) = map_node {
                    mn.collect_idents(set);
                }
            }
            Node::Unary { node, .. } => node.collect_idents(set),
            Node::Operation { left, right, .. } => {
                left.collect_idents(set);
                right.collect_idents(set);
            }
            Node::Postfix { node, operator } => {
                node.collect_idents(set);
                operator.collect_idents(set);
            }
        }
    }

    /// Count the total number of nodes in this subtree. Useful for measuring
    /// AST size reduction from optimization.
    pub fn node_count(&self) -> usize {
        1 + match self {
            Node::Value(_) | Node::Ident(_) => 0,
            Node::Array(items) => items.iter().map(|i| i.node_count()).sum(),
            Node::Range(start, end) => start.node_count() + end.node_count(),
            Node::Func { args, .. } => args.iter().map(|a| a.node_count()).sum(),
            Node::Unary { node, .. } => node.node_count(),
            Node::Operation { left, right, .. } => left.node_count() + right.node_count(),
            Node::Postfix { node, operator } => {
                node.node_count() + operator_children_count(operator)
            }
        }
    }
}

fn operator_children_count(op: &PostfixOperator) -> usize {
    match op {
        PostfixOperator::Index { idx, .. } => idx.node_count(),
        PostfixOperator::Default(n) | PostfixOperator::Pipe(n) => n.node_count(),
        PostfixOperator::Ternary { left, right } => left.node_count() + right.node_count(),
        PostfixOperator::Range(..) => 0,
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::Value(Value::default())
    }
}

impl From<Pairs<'_, Rule>> for Node {
    fn from(pairs: Pairs<Rule>) -> Self {
        PRATT_PARSER
            .map_primary(|primary| primary.into())
            .map_prefix(|operator, right| Node::Unary {
                operator: operator.into(),
                node: Box::new(right),
            })
            .map_postfix(|left, operator| Node::Postfix {
                operator: operator.into(),
                node: Box::new(left),
            })
            .map_infix(|left, operator, right| Node::Operation {
                operator: operator.into(),
                left: Box::new(left),
                right: Box::new(right),
            })
            .parse(pairs)
    }
}

impl From<Pair<'_, Rule>> for Node {
    fn from(pair: Pair<Rule>) -> Self {
        trace!("{:?} = {}", &pair.as_rule(), pair.as_str());
        match pair.as_rule() {
            Rule::expr => pair.into_inner().into(),
            Rule::value => Node::Value(pair.into_inner().into()),
            Rule::ident => Node::Ident(pair.as_str().to_string()),
            Rule::func => {
                let mut inner = pair.into_inner();
                let ident = inner.next().unwrap().as_str().to_string();
                let mut predicate = None;
                let mut args: Vec<Node> = Vec::new();
                for arg in inner {
                    match arg.as_rule() {
                        Rule::predicate => {
                            predicate = Some(Box::new(arg.into_inner().into()));
                        },
                        _ => {
                            args.push(arg.into());
                        },
                    }
                }
                // If no explicit predicate was parsed but the last arg references `#`,
                // promote it to the predicate. This matches Go expr-lang behavior where
                // `filter(arr, # > 2)` works without braces around the predicate.
                // Only applies to known predicate-accepting functions to avoid breaking
                // nested calls like `indexOf("abc", #)` inside braced predicates.
                // Uses >= 1 (not >= 2) so pipe syntax works: `arr | filter(# > 2)`.
                if predicate.is_none()
                    && !args.is_empty()
                    && Self::accepts_predicate(&ident)
                    && args.last().unwrap().contains_hash_ident()
                {
                    let last = args.pop().unwrap();
                    predicate = Some(Box::new(Program {
                        lines: Vec::new(),
                        expr: last,
                    }));
                }
                Node::Func { ident, args, predicate, threshold: None, throws: false, map_node: None }
            },
            Rule::array => Node::Array(pair.into_inner().map(|p| p.into()).collect()),
            Rule::map => {
                let mut map = IndexMap::new();
                let vals = pair.clone();
                for (key, val) in pair
                    .into_inner()
                    .step_by(2)
                    .zip(vals.into_inner().skip(1).step_by(2))
                {
                    let key = key.as_str().to_string();
                    map.insert(key, val.into_inner().into());
                }
                Node::Value(Value::Map(map))
            }
            Rule::range => {
                let mut inner = pair.into_inner();
                let start = Box::new(inner.next().unwrap().into());
                let end = Box::new(inner.next().unwrap().into());
                Node::Range(start, end)
            }
            rule => unreachable!("Unexpected rule: {rule:?}"),
        }
    }
}
