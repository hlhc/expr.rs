use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::Value;

/// Converts `count(arr, pred) > 0` and `count(arr, pred) >= 1` to `any(arr, pred)`.
/// `any` terminates early on first match, unlike `count` which must scan everything.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Operation {
        operator, left, right,
    } = node
    {
        let threshold = match operator {
            Operator::GreaterThan => {
                matches!(right.as_ref(), Node::Value(Value::Number(0)))
            }
            Operator::GreaterThanOrEqual => {
                matches!(right.as_ref(), Node::Value(Value::Number(1)))
            }
            _ => false,
        };

        if threshold
            && let Node::Func { ident, args, predicate, .. } = left.as_ref()
            && ident == "count"
            && args.len() == 1
        {
            *node = Node::Func {
                ident: "any".to_string(),
                args: args.clone(),
                predicate: predicate.clone(),
                threshold: None,
                throws: false,
                map_node: None,
            };
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result};
    use crate::ast::node::Node;
    use crate::ast::operator::Operator;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn count_any_gt_zero() -> Result<()> {
        assert_eq!(
            eval("count([1, 2, 3], {# > 1}) > 0", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("count([1, 2, 3], {# > 5}) >= 1", &Context::default())?.to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn ast_count_any_converts_to_any() {
        let mut n = Node::Operation {
            operator: Operator::GreaterThan,
            left: Box::new(Node::Func {
                ident: "count".into(),
                args: vec![Node::Array(vec![num(1), num(2)])],
                predicate: None,
                threshold: None,
                throws: false,
                map_node: None,
            }),
            right: Box::new(num(0)),
        };
        let optimized = optimize_node(&mut n);
        assert!(matches!(optimized, Node::Func { ident, .. }
            if ident == "any"));
    }
}
