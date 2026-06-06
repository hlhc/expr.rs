use indexmap::IndexMap;

use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::{MapKey, Value};

/// Converts `x in [1, 2, 3]` to `x in {1: true, 2: true, 3: true}` for O(1) lookup.
/// Only applies when all array elements are the same type (all integers or all strings).
#[allow(dead_code)]
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Operation {
        operator: Operator::In,
        left,
        right,
    } = node
        && let Node::Array(items) = right.as_ref()
    {
        if items.is_empty() {
            return false;
        }
        // Try integer set
        if items
            .iter()
            .all(|v| matches!(v, Node::Value(Value::Number(_))))
        {
            let mut map = IndexMap::new();
            for v in items {
                if let Node::Value(Value::Number(n)) = v {
                    map.insert(MapKey::Number(*n), Value::Bool(true));
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
        if items
            .iter()
            .all(|v| matches!(v, Node::Value(Value::String(_))))
        {
            let mut map = IndexMap::new();
            for v in items {
                if let Node::Value(Value::String(s)) = v {
                    map.insert(MapKey::String(s.clone()), Value::Bool(true));
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
    false
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{num, optimize_node};
    use crate::ast::node::Node;
    use crate::ast::operator::Operator;
    use crate::{Context, Result, eval};

    #[test]
    fn in_array_integer_set() -> Result<()> {
        assert_eq!(
            eval("3 in [1, 2, 3]", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("4 in [1, 2, 3]", &Context::default())?.to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn in_array_string_set() -> Result<()> {
        assert_eq!(
            eval(r#""b" in ["a", "b", "c"]"#, &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval(r#""d" in ["a", "b", "c"]"#, &Context::default())?.to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn ast_in_array_preserved() {
        let mut n = Node::Operation {
            operator: Operator::In,
            left: Box::new(num(3)),
            right: Box::new(Node::Array(vec![num(1), num(2), num(3)])),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(
            n, original,
            "x in [a,b,c] must not be rewritten to map - type-mixing unsafe"
        );
    }
}
