use crate::ast::node::Node;

/// Converts `sum(map(arr, pred))` to `sum(arr, pred)`, passing the predicate directly
/// to sum for more efficient evaluation (no intermediate array allocation).
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Func { ident, args, .. } = node
        && ident == "sum"
        && args.len() == 1
        && let Node::Func { ident: inner_ident, args: inner_args, predicate, .. } = &args[0]
        && inner_ident == "map"
        && inner_args.len() == 1
    {
        *node = Node::Func {
            ident: "sum".to_string(),
            args: vec![inner_args[0].clone()],
            predicate: predicate.clone(),
            threshold: None,
            throws: false,
            map_node: None,
        };
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result};
    use crate::ast::node::Node;
    use crate::ast::operator::Operator;
    use crate::ast::program::Program;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn sum_map_combines_predicate() -> Result<()> {
        assert_eq!(
            eval("sum(map([1, 2, 3], {# * 2}))", &Context::default())?.to_string(),
            "12"
        );
        Ok(())
    }

    #[test]
    fn ast_sum_map_preserves_predicate() {
        let mut n = Node::Func {
            ident: "sum".into(),
            args: vec![Node::Func {
                ident: "map".into(),
                args: vec![Node::Array(vec![num(1), num(2), num(3)])],
                predicate: Some(Box::new(Program {
                    lines: vec![],
                    expr: Node::Operation {
                        operator: Operator::Multiply,
                        left: Box::new(Node::Ident("#".into())),
                        right: Box::new(num(2)),
                    },
                })),
                threshold: None,
                throws: false,
                map_node: None,
            }],
            predicate: None,
            threshold: None,
            throws: false,
            map_node: None,
        };
        let optimized = optimize_node(&mut n);
        match &optimized {
            Node::Func { ident, args, predicate, .. } => {
                assert_eq!(ident, "sum");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Node::Array(..)));
                assert!(predicate.is_some(), "predicate should be preserved from inner map");
            }
            other => panic!("Expected Func node, got {other:?}"),
        }
    }
}
