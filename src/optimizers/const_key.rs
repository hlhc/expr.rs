use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;
use crate::{MapKey, Value};

pub fn optimize(node: &mut Node) -> bool {
    if let Node::Postfix {
        operator: PostfixOperator::Index { idx, optional: _ },
        node: container,
    } = node
    {
        match container.as_ref() {
            Node::Value(Value::Map(map)) => {
                let key = index_key_to_map_key(idx.as_ref());
                if let Some(k) = key {
                    *node = Node::Value(map.get(&k).cloned().unwrap_or(Value::Nil));
                    return true;
                }
            }
            Node::Value(Value::Array(arr)) => {
                if let Node::Value(Value::Number(i)) = idx.as_ref() {
                    let i = idx_to_usize(*i, arr.len());
                    *node = Node::Value(arr.get(i).cloned().unwrap_or(Value::Nil));
                    return true;
                }
            }
            Node::Array(arr) if arr.iter().all(|e| matches!(e, Node::Value(_))) => {
                if let Node::Value(Value::Number(i)) = idx.as_ref() {
                    let i = idx_to_usize(*i, arr.len());
                    if let Some(Node::Value(v)) = arr.get(i) {
                        *node = Node::Value(v.clone());
                        return true;
                    }
                }
            }
            Node::Range(start, end) => {
                if let (Node::Value(Value::Number(s)), Node::Value(Value::Number(e))) =
                    (start.as_ref(), end.as_ref())
                    && let Node::Value(Value::Number(i)) = idx.as_ref()
                {
                    let length = (e - s + 1) as usize;
                    let pos = idx_to_usize(*i, length);
                    *node = Node::Value(Value::Number(s + pos as i64));
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn idx_to_usize(i: i64, len: usize) -> usize {
    if i < 0 {
        (len as i64 + i) as usize
    } else {
        i as usize
    }
}

fn index_key_to_map_key(idx: &Node) -> Option<MapKey> {
    match idx {
        Node::Value(Value::String(s)) => Some(MapKey::String(s.clone())),
        Node::Value(Value::Number(n)) => Some(MapKey::Number(*n)),
        Node::Ident(id) => Some(MapKey::String(id.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{num, optimize_node};
    use crate::ast::node::Node;
    use crate::ast::postfix_operator::PostfixOperator;
    use crate::{Context, Result, Value, eval};

    #[test]
    fn map_key_lookup() -> Result<()> {
        assert_eq!(
            eval(r#"{foo: "bar"}.foo"#, &Context::default())?.to_string(),
            r#""bar""#
        );
        assert_eq!(
            eval(r#"{foo: "bar"}["foo"]"#, &Context::default())?.to_string(),
            r#""bar""#
        );
        Ok(())
    }

    #[test]
    fn array_index_lookup() -> Result<()> {
        assert_eq!(
            eval(r#"["a", "b", "c"][0]"#, &Context::default())?.to_string(),
            r#""a""#
        );
        assert_eq!(
            eval(r#"["a", "b", "c"][1]"#, &Context::default())?.to_string(),
            r#""b""#
        );
        assert_eq!(
            eval(r#"["a", "b", "c"][-1]"#, &Context::default())?.to_string(),
            r#""c""#
        );
        Ok(())
    }

    #[test]
    fn range_index_lookup() -> Result<()> {
        assert_eq!(eval("(3..5)[0]", &Context::default())?.to_string(), "3");
        assert_eq!(eval("(3..5)[1]", &Context::default())?.to_string(), "4");
        assert_eq!(eval("(3..5)[2]", &Context::default())?.to_string(), "5");
        assert_eq!(eval("(3..5)[-1]", &Context::default())?.to_string(), "5");
        Ok(())
    }

    #[test]
    fn ast_range_index_folds() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Index {
                idx: Box::new(num(1)),
                optional: false,
            },
            node: Box::new(Node::Range(Box::new(num(3)), Box::new(num(5)))),
        };
        let optimized = optimize_node(&mut n);
        assert_eq!(optimized, Node::Value(Value::Number(4)));
    }

    #[test]
    fn ast_array_of_values_index_folds() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Index {
                idx: Box::new(num(1)),
                optional: false,
            },
            node: Box::new(Node::Array(vec![num(10), num(20), num(30)])),
        };
        let optimized = optimize_node(&mut n);
        assert_eq!(optimized, num(20));
    }
}
