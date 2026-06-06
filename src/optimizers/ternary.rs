use crate::Value;
use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;

pub fn optimize(node: &mut Node) -> bool {
    if let Node::Postfix {
        operator: PostfixOperator::Ternary { left, right },
        node: condition,
    } = node
    {
        match condition.as_ref() {
            Node::Value(Value::Bool(true)) => {
                *node = *left.clone();
                return true;
            }
            Node::Value(Value::Bool(false)) => {
                *node = *right.clone();
                return true;
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{num, optimize_node};
    use crate::ast::node::Node;
    use crate::ast::postfix_operator::PostfixOperator;
    use crate::{Context, Result, eval};

    #[test]
    fn ternary_constant_condition() -> Result<()> {
        assert_eq!(eval("true ? 1 : 2", &Context::default())?.to_string(), "1");
        assert_eq!(eval("false ? 1 : 2", &Context::default())?.to_string(), "2");
        Ok(())
    }

    #[test]
    fn ast_ternary_true() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Ternary {
                left: Box::new(num(1)),
                right: Box::new(num(2)),
            },
            node: Box::new(super::super::test_helpers::bool_val(true)),
        };
        assert_eq!(optimize_node(&mut n), num(1));
    }

    #[test]
    fn ast_ternary_false() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Ternary {
                left: Box::new(num(1)),
                right: Box::new(num(2)),
            },
            node: Box::new(super::super::test_helpers::bool_val(false)),
        };
        assert_eq!(optimize_node(&mut n), num(2));
    }
}
