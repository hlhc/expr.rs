use indexmap::IndexMap;

use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::Value;

/// Converts `x in [1, 2, 3]` to `x in {1: true, 2: true, 3: true}` for O(1) lookup.
/// Only applies when all array elements are the same type (all integers or all strings).
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Operation {
        operator: Operator::In,
        left,
        right,
    } = node
    {
        if let Node::Array(items) = right.as_ref() {
            if items.is_empty() {
                return false;
            }
            // Try integer set
            if items.iter().all(|v| matches!(v, Node::Value(Value::Number(_)))) {
                let mut map = IndexMap::new();
                for v in items {
                    if let Node::Value(Value::Number(n)) = v {
                        map.insert(n.to_string(), Value::Bool(true));
                    }
                }
                *node = Node::Operation {
                    operator: Operator::In,
                    left: left.clone(),
                    right: Box::new(Node::Value(Value::Map(map))),
                };
                return true;
            }
            // Try string set
            if items.iter().all(|v| matches!(v, Node::Value(Value::String(_)))) {
                let mut map = IndexMap::new();
                for v in items {
                    if let Node::Value(Value::String(s)) = v {
                        map.insert(s.clone(), Value::Bool(true));
                    }
                }
                *node = Node::Operation {
                    operator: Operator::In,
                    left: left.clone(),
                    right: Box::new(Node::Value(Value::Map(map))),
                };
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result, Value};
    use crate::ast::node::Node;
    use crate::ast::operator::Operator;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn in_array_integer_set() -> Result<()> {
        assert_eq!(eval("3 in [1, 2, 3]", &Context::default())?.to_string(), "true");
        assert_eq!(eval("4 in [1, 2, 3]", &Context::default())?.to_string(), "false");
        Ok(())
    }

    #[test]
    fn in_array_string_set() -> Result<()> {
        assert_eq!(eval(r#""b" in ["a", "b", "c"]"#, &Context::default())?.to_string(), "true");
        assert_eq!(eval(r#""d" in ["a", "b", "c"]"#, &Context::default())?.to_string(), "false");
        Ok(())
    }

    #[test]
    fn ast_in_array_converts_to_map() {
        let mut n = Node::Operation {
            operator: Operator::In,
            left: Box::new(num(3)),
            right: Box::new(Node::Array(vec![
                num(1),
                num(2),
                num(3),
            ])),
        };
        let optimized = optimize_node(&mut n);
        let map = match &optimized {
            Node::Operation { operator: Operator::In, right, .. } => match right.as_ref() {
                Node::Value(Value::Map(m)) => m.clone(),
                other => panic!("Expected Map, got {other:?}"),
            },
            other => panic!("Expected In operation, got {other:?}"),
        };
        assert_eq!(map.get("1"), Some(&Value::Bool(true)));
        assert_eq!(map.get("2"), Some(&Value::Bool(true)));
        assert_eq!(map.get("3"), Some(&Value::Bool(true)));
    }
}
