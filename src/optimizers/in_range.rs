use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::Value;

/// Converts `x in m..n` to `x >= m && x <= n` when m and n are integer constants.
/// This enables further constant folding if x is also a constant.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Operation {
        operator: Operator::In,
        left,
        right,
    } = node
    {
        if let Node::Range(start, end) = right.as_ref()
            && let (Node::Value(Value::Number(m)), Node::Value(Value::Number(n))) =
                (start.as_ref(), end.as_ref())
        {
            *node = Node::Operation {
                operator: Operator::And,
                left: Box::new(Node::Operation {
                    operator: Operator::GreaterThanOrEqual,
                    left: left.clone(),
                    right: Box::new(Node::Value(Value::Number(*m))),
                }),
                right: Box::new(Node::Operation {
                    operator: Operator::LessThanOrEqual,
                    left: left.clone(),
                    right: Box::new(Node::Value(Value::Number(*n))),
                }),
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
    use super::super::test_helpers::{num, bool_val, optimize_node};

    #[test]
    fn in_range_conversion() -> Result<()> {
        assert_eq!(eval("3 in 1..5", &Context::default())?.to_string(), "true");
        assert_eq!(eval("0 in 1..5", &Context::default())?.to_string(), "false");
        assert_eq!(eval("6 in 1..5", &Context::default())?.to_string(), "false");
        Ok(())
    }

    #[test]
    fn in_range_with_variable() -> Result<()> {
        let ctx = Context::from_iter([("x", 3)]);
        assert_eq!(eval("x in 1..5", &ctx)?.to_string(), "true");
        Ok(())
    }

    #[test]
    fn ast_in_range_converts_to_comparison() {
        let mut n = Node::Operation {
            operator: Operator::In,
            left: Box::new(num(3)),
            right: Box::new(Node::Range(Box::new(num(1)), Box::new(num(5)))),
        };
        let optimized = optimize_node(&mut n);
        assert_eq!(optimized, bool_val(true));
    }
}
