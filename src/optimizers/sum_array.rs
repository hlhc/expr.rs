use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::Value;

/// Converts `sum([a, b, c, ...])` to `a + b + c + ...` when the array has 2+ constant elements.
/// This enables further constant folding.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Func { ident, args, predicate, .. } = node
        && ident == "sum"
        && args.len() == 1
        && predicate.is_none()
    {
        if let Node::Array(items) = &args[0] {
            if items.len() >= 2 {
                let all_numeric = items.iter().all(|v| matches!(v, Node::Value(Value::Number(_) | Value::Float(_))));
                if all_numeric {
                    let values: Vec<Value> = items.iter().map(|v| {
                        match v {
                            Node::Value(val) => val.clone(),
                            _ => unreachable!(),
                        }
                    }).collect();
                    *node = fold_sum_array(&values);
                    return true;
                }
            }
        }
    }
    false
}

fn fold_sum_array(arr: &[Value]) -> Node {
    if arr.len() > 2 {
        Node::Operation {
            operator: Operator::Add,
            left: Box::new(Node::Value(arr[0].clone())),
            right: Box::new(fold_sum_array(&arr[1..])),
        }
    } else if arr.len() == 2 {
        Node::Operation {
            operator: Operator::Add,
            left: Box::new(Node::Value(arr[0].clone())),
            right: Box::new(Node::Value(arr[1].clone())),
        }
    } else {
        Node::Value(Value::Nil)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result};
    use crate::ast::node::Node;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn sum_array_folds_to_addition() -> Result<()> {
        assert_eq!(eval("sum([1, 2, 3, 4])", &Context::default())?.to_string(), "10");
        Ok(())
    }

    #[test]
    fn sum_array_two_elements() -> Result<()> {
        assert_eq!(eval("sum([5, 7])", &Context::default())?.to_string(), "12");
        Ok(())
    }

    #[test]
    fn ast_sum_array_folds() {
        let mut n = Node::Func {
            ident: "sum".into(),
            args: vec![Node::Array(vec![
                num(1),
                num(2),
                num(3),
            ])],
            predicate: None,
            threshold: None,
            throws: false,
            map_node: None,
        };
        let optimized = optimize_node(&mut n);
        assert_eq!(optimized, num(6));
    }
}
