use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;

pub fn optimize(node: &mut Node) -> bool {
    if let Node::Postfix {
        operator: PostfixOperator::Default(_),
        node: inner,
    } = node
        && let Node::Value(v) = inner.as_ref()
        && !v.is_nil()
    {
        *node = *inner.clone();
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result, Value};
    use crate::ast::node::Node;
    use crate::ast::postfix_operator::PostfixOperator;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn nil_coalesce_non_nil() -> Result<()> {
        assert_eq!(eval("5 ?? 10", &Context::default())?.to_string(), "5");
        assert_eq!(eval(r#""hi" ?? "bye""#, &Context::default())?.to_string(), r#""hi""#);
        Ok(())
    }

    #[test]
    fn nil_coalesce_nil() -> Result<()> {
        assert_eq!(eval("nil ?? 10", &Context::default())?.to_string(), "10");
        Ok(())
    }

    #[test]
    fn ast_nil_coalesce_non_nil() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Default(Box::new(num(10))),
            node: Box::new(num(5)),
        };
        assert_eq!(optimize_node(&mut n), num(5));
    }

    #[test]
    fn ast_nil_coalesce_nil_keeps_default() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Default(Box::new(num(10))),
            node: Box::new(Node::Value(Value::Nil)),
        };
        let optimized = optimize_node(&mut n);
        assert!(matches!(optimized, Node::Postfix {
            operator: PostfixOperator::Default(_),
            node: _,
        }));
    }
}
