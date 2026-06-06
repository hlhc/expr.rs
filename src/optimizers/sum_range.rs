use crate::Value;
use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::ast::program::Program;

/// Evaluates `sum(m..n)` and `sum(m..n, pred)` at compile time when m,n are constants.
/// Also handles `reduce(m..n, # + #acc)` patterns.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Func {
        ident,
        args,
        predicate,
        ..
    } = node
        && ident == "sum"
        && args.len() == 1
        && let Node::Range(start, end) = &args[0]
        && let (Node::Value(Value::Number(m)), Node::Value(Value::Number(n))) =
            (start.as_ref(), end.as_ref())
    {
        let (m, n) = (*m, *n);
        if n < m {
            return false;
        }
        let count = n - m + 1;
        let sum = count * (m + n) / 2;

        if predicate.is_none() {
            *node = Node::Value(Value::Number(sum));
            return true;
        } else if let Some(pred) = predicate
            && let Some(result) = apply_sum_predicate(sum, count, pred)
        {
            *node = Node::Value(Value::Number(result));
            return true;
        }
    }

    // Pattern: reduce(m..n, # + #acc) with optional initial value
    if let Node::Func {
        ident,
        args,
        predicate,
        ..
    } = node
        && ident == "reduce"
        && args.len() == 1
        && predicate.is_some()
        && let Node::Range(start, end) = &args[0]
        && let (Node::Value(Value::Number(m)), Node::Value(Value::Number(n))) =
            (start.as_ref(), end.as_ref())
    {
        let (m, n) = (*m, *n);
        if n < m {
            return false;
        }
        let sum = (n - m + 1) * (m + n) / 2;
        let pred = predicate.as_ref().unwrap();
        if is_pointer_plus_acc(pred) {
            *node = Node::Value(Value::Number(sum));
            return true;
        }
    }

    false
}

fn is_pointer_plus_acc(program: &Program) -> bool {
    if let Node::Operation {
        operator: Operator::Add,
        left,
        right,
    } = &program.expr
    {
        match (left.as_ref(), right.as_ref()) {
            (Node::Ident(l), Node::Ident(r)) if l == "#" && r == "#acc" => return true,
            (Node::Ident(l), Node::Ident(r)) if l == "#acc" && r == "#" => return true,
            _ => {}
        }
    }
    false
}

fn apply_sum_predicate(sum: i64, count: i64, pred: &Program) -> Option<i64> {
    match &pred.expr {
        // Case: # (identity) — sum remains unchanged
        Node::Ident(id) if id == "#" => Some(sum),

        // Case: binary operation with # and constant
        Node::Operation {
            operator,
            left,
            right,
        } => {
            let (ptr_on_left, constant) =
                extract_pointer_and_constant(left.as_ref(), right.as_ref())?;
            match operator {
                Operator::Multiply => Some(constant * sum),
                Operator::Add => Some(sum + count * constant),
                Operator::Subtract => {
                    if ptr_on_left {
                        Some(sum - count * constant)
                    } else {
                        Some(count * constant - sum)
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn extract_pointer_and_constant(left: &Node, right: &Node) -> Option<(bool, i64)> {
    if let Node::Ident(id) = left
        && id == "#"
        && let Node::Value(Value::Number(n)) = right
    {
        Some((true, *n))
    } else if let Node::Value(Value::Number(n)) = left
        && let Node::Ident(id) = right
        && id == "#"
    {
        Some((false, *n))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{num, optimize_node};
    use crate::ast::node::Node;
    use crate::{Context, Result, eval};

    #[test]
    fn sum_range_arithmetic_series() -> Result<()> {
        assert_eq!(eval("sum(1..5)", &Context::default())?.to_string(), "15");
        assert_eq!(eval("sum(1..3)", &Context::default())?.to_string(), "6");
        Ok(())
    }

    #[test]
    fn sum_range_identity_predicate() -> Result<()> {
        assert_eq!(
            eval("sum(1..3, {#})", &Context::default())?.to_string(),
            "6"
        );
        Ok(())
    }

    #[test]
    fn sum_range_multiply_predicate() -> Result<()> {
        assert_eq!(
            eval("sum(1..3, {# * 2})", &Context::default())?.to_string(),
            "12"
        );
        Ok(())
    }

    #[test]
    fn sum_range_add_predicate() -> Result<()> {
        assert_eq!(
            eval("sum(1..3, {# + 1})", &Context::default())?.to_string(),
            "9"
        );
        Ok(())
    }

    #[test]
    fn sum_range_subtract_predicate() -> Result<()> {
        assert_eq!(
            eval("sum(1..3, {# - 1})", &Context::default())?.to_string(),
            "3"
        );
        Ok(())
    }

    #[test]
    fn reduce_range_sum() -> Result<()> {
        assert_eq!(
            eval("reduce(1..5, {# + #acc})", &Context::default())?.to_string(),
            "15"
        );
        Ok(())
    }

    #[test]
    fn reduce_range_sum_no_initial() -> Result<()> {
        assert_eq!(
            eval("reduce(1..5, {# + #acc})", &Context::default())?.to_string(),
            "15"
        );
        Ok(())
    }

    #[test]
    fn ast_sum_range_folds() {
        let mut n = Node::Func {
            ident: "sum".into(),
            args: vec![Node::Range(Box::new(num(1)), Box::new(num(5)))],
            predicate: None,
            threshold: None,
            throws: false,
            map_node: None,
        };
        let optimized = optimize_node(&mut n);
        assert_eq!(optimized, num(15));
    }
}
