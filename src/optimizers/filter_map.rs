use crate::ast::node::Node;

/// Converts `map(filter(arr, p1), p2)` to `filter(arr, p1)` with map_node = p2
/// when p2 does not reference `#` as an index pointer.
/// The filter runtime can then apply p2 to each matching element directly.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Func {
        ident, args, predicate, ..
    } = node
        && ident == "map"
        && args.len() == 1
        && predicate.is_some()
    {
        if let Node::Func {
            ident: inner_ident,
            args: inner_args,
            predicate: inner_predicate,
            ..
        } = &args[0]
            && inner_ident == "filter"
            && inner_args.len() == 1
        {
            // Check that predicate doesn't reference # as index pointer
            let map_pred = predicate.as_ref().unwrap();
            if !contains_index_pointer(&map_pred.expr) {
                let map_node = map_pred.expr.clone();
                *node = Node::Func {
                    ident: "filter".to_string(),
                    args: inner_args.clone(),
                    predicate: inner_predicate.clone(),
                    threshold: None,
                    throws: false,
                    map_node: Some(Box::new(map_node)),
                };
                return true;
            }
        }
    }
    false
}

fn contains_index_pointer(node: &Node) -> bool {
    match node {
        Node::Ident(id) => id == "#",
        Node::Array(items) => items.iter().any(contains_index_pointer),
        Node::Range(start, end) => {
            contains_index_pointer(start) || contains_index_pointer(end)
        }
        Node::Value(_) => false,
        Node::Func { args, .. } => args.iter().any(contains_index_pointer),
        Node::Unary { node: inner, .. } => contains_index_pointer(inner),
        Node::Operation { left, right, .. } => {
            contains_index_pointer(left) || contains_index_pointer(right)
        }
        Node::Postfix { node: inner, operator } => {
            contains_index_pointer(inner) || operator_contains_index(operator)
        }
    }
}

fn operator_contains_index(op: &crate::ast::postfix_operator::PostfixOperator) -> bool {
    match op {
        crate::ast::postfix_operator::PostfixOperator::Index { idx, .. } => {
            contains_index_pointer(idx)
        }
        crate::ast::postfix_operator::PostfixOperator::Default(n)
        | crate::ast::postfix_operator::PostfixOperator::Pipe(n) => {
            contains_index_pointer(n)
        }
        crate::ast::postfix_operator::PostfixOperator::Ternary { left, right } => {
            contains_index_pointer(left) || contains_index_pointer(right)
        }
        crate::ast::postfix_operator::PostfixOperator::Range(..) => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result};
    use crate::ast::node::Node;
    use crate::ast::operator::Operator;
    use crate::ast::program::Program;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn filter_map_optimization() -> Result<()> {
        assert_eq!(
            eval("map(filter([1, 2, 3], {# > 1}), {# * 10})", &Context::default())?.to_string(),
            "[20, 30]"
        );
        Ok(())
    }

    #[test]
    fn ast_filter_map_merges_without_hash() {
        let mut n = Node::Func {
            ident: "map".into(),
            args: vec![Node::Func {
                ident: "filter".into(),
                args: vec![Node::Array(vec![num(1), num(2), num(3)])],
                predicate: Some(Box::new(Program {
                    lines: vec![],
                    expr: Node::Operation {
                        operator: Operator::GreaterThan,
                        left: Box::new(Node::Ident("#".into())),
                        right: Box::new(num(1)),
                    },
                })),
                threshold: None,
                throws: false,
                map_node: None,
            }],
            predicate: Some(Box::new(Program {
                lines: vec![],
                expr: num(42),
            })),
            threshold: None,
            throws: false,
            map_node: None,
        };
        let optimized = optimize_node(&mut n);
        match &optimized {
            Node::Func { ident, map_node, args, .. } => {
                assert_eq!(ident, "filter", "should become filter");
                assert_eq!(args.len(), 1);
                assert!(map_node.is_some(), "map_node should be set");
                assert_eq!(*map_node.clone().unwrap(), num(42));
            }
            other => panic!("Expected Func node, got {other:?}"),
        }
    }

    #[test]
    fn ast_filter_map_skips_hash_in_map_predicate() {
        let mut n = Node::Func {
            ident: "map".into(),
            args: vec![Node::Func {
                ident: "filter".into(),
                args: vec![Node::Array(vec![num(1), num(2), num(3)])],
                predicate: Some(Box::new(Program {
                    lines: vec![],
                    expr: Node::Operation {
                        operator: Operator::GreaterThan,
                        left: Box::new(Node::Ident("#".into())),
                        right: Box::new(num(1)),
                    },
                })),
                threshold: None,
                throws: false,
                map_node: None,
            }],
            predicate: Some(Box::new(Program {
                lines: vec![],
                expr: Node::Operation {
                    operator: Operator::Multiply,
                    left: Box::new(Node::Ident("#".into())),
                    right: Box::new(num(10)),
                },
            })),
            threshold: None,
            throws: false,
            map_node: None,
        };
        let optimized = optimize_node(&mut n);
        match &optimized {
            Node::Func { ident, .. } => {
                assert_eq!(ident, "map", "should stay as map when predicate references #");
            }
            other => panic!("Expected Func node, got {other:?}"),
        }
    }
}
