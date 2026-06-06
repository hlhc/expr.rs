use crate::Value;
use crate::ast::node::Node;
use crate::ast::operator::Operator;

/// Sets a threshold on `count` so it can exit early once enough matches are found.
///   count(arr, pred) > N  → threshold = N + 1
///   count(arr, pred) >= N → threshold = N
///   count(arr, pred) < N  → threshold = N
///   count(arr, pred) <= N → threshold = N + 1
///
/// Skips thresholds ≤ 1 which are handled by count_any.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Operation {
        operator,
        left,
        right,
    } = node
    {
        let count_node: &Node = left.as_ref();
        let threshold_val = match operator {
            Operator::GreaterThan => {
                if let Node::Value(Value::Number(n)) = right.as_ref() {
                    Some(n + 1)
                } else {
                    None
                }
            }
            Operator::GreaterThanOrEqual => {
                if let Node::Value(Value::Number(n)) = right.as_ref() {
                    Some(*n)
                } else {
                    None
                }
            }
            Operator::LessThan => {
                if let Node::Value(Value::Number(n)) = right.as_ref() {
                    Some(*n)
                } else {
                    None
                }
            }
            Operator::LessThanOrEqual => {
                if let Node::Value(Value::Number(n)) = right.as_ref() {
                    Some(n + 1)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(t) = threshold_val
            && t > 1
            && let Node::Func {
                ident,
                args,
                predicate,
                ..
            } = count_node
            && ident == "count"
            && args.len() == 1
        {
            let updated = Node::Func {
                ident: "count".to_string(),
                args: args.clone(),
                predicate: predicate.clone(),
                threshold: Some(t),
                throws: false,
                map_node: None,
            };
            **left = updated;
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
    fn count_threshold_gt() -> Result<()> {
        assert_eq!(
            eval("count([1, 2, 3], {# > 0}) > 2", &Context::default())?.to_string(),
            "true"
        );
        Ok(())
    }

    #[test]
    fn count_threshold_gte() -> Result<()> {
        assert_eq!(
            eval("count([1, 2, 3, 4], {# > 2}) >= 2", &Context::default())?.to_string(),
            "true"
        );
        Ok(())
    }

    #[test]
    fn count_threshold_lt() -> Result<()> {
        assert_eq!(
            eval("count([1, 2, 3, 4], {# > 2}) < 3", &Context::default())?.to_string(),
            "true"
        );
        Ok(())
    }

    #[test]
    fn count_threshold_lte() -> Result<()> {
        assert_eq!(
            eval("count([1, 2, 3, 4], {# > 2}) <= 2", &Context::default())?.to_string(),
            "true"
        );
        Ok(())
    }

    fn make_count_op(n: i64, op: Operator) -> Node {
        Node::Operation {
            operator: op,
            left: Box::new(Node::Func {
                ident: "count".into(),
                args: vec![Node::Array(vec![num(1), num(2), num(3)])],
                predicate: None,
                threshold: None,
                throws: false,
                map_node: None,
            }),
            right: Box::new(num(n)),
        }
    }

    #[test]
    fn ast_count_threshold_gt_sets_n_plus_1() {
        let mut n = make_count_op(2, Operator::GreaterThan);
        let optimized = optimize_node(&mut n);
        let threshold = extract_count_threshold(&optimized);
        assert_eq!(threshold, Some(3), "count > 2 → threshold = 3");
    }

    #[test]
    fn ast_count_threshold_gte_sets_n() {
        let mut n = make_count_op(2, Operator::GreaterThanOrEqual);
        let optimized = optimize_node(&mut n);
        let threshold = extract_count_threshold(&optimized);
        assert_eq!(threshold, Some(2), "count >= 2 → threshold = 2");
    }

    #[test]
    fn ast_count_threshold_lt_sets_n() {
        let mut n = make_count_op(3, Operator::LessThan);
        let optimized = optimize_node(&mut n);
        let threshold = extract_count_threshold(&optimized);
        assert_eq!(threshold, Some(3), "count < 3 → threshold = 3");
    }

    #[test]
    fn ast_count_threshold_lte_sets_n_plus_1() {
        let mut n = make_count_op(3, Operator::LessThanOrEqual);
        let optimized = optimize_node(&mut n);
        let threshold = extract_count_threshold(&optimized);
        assert_eq!(threshold, Some(4), "count <= 3 → threshold = 4");
    }

    #[test]
    fn ast_count_threshold_skips_at_most_1() {
        let mut n = make_count_op(0, Operator::GreaterThan);
        let optimized = optimize_node(&mut n);
        let threshold = extract_count_threshold(&optimized);
        assert_eq!(
            threshold, None,
            "count > 0 → threshold should NOT be set (threshold = 1 skipped)"
        );
    }

    fn extract_count_threshold(node: &Node) -> Option<i64> {
        match node {
            Node::Operation { left, .. } => match left.as_ref() {
                Node::Func { threshold, .. } => *threshold,
                _ => None,
            },
            _ => None,
        }
    }
}
