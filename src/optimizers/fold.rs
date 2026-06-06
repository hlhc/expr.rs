use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::ast::unary_operator::UnaryOperator;
use crate::{MapKey, Value};

pub fn optimize(node: &mut Node) -> bool {
    let mut changed = false;
    changed |= try_constant_fold(node);
    changed |= try_boolean_minimization(node);
    changed
}

// ---- Constant Folding ----

fn try_constant_fold(node: &mut Node) -> bool {
    match node {
        Node::Operation {
            operator,
            left,
            right,
        } => {
            if let (Node::Value(l), Node::Value(r)) = (left.as_ref(), right.as_ref())
                && let Some(result) = fold_binary_op(operator, l.clone(), r.clone())
            {
                *node = Node::Value(result);
                return true;
            }
        }
        Node::Unary {
            operator,
            node: inner,
        } => {
            if let Node::Value(v) = inner.as_ref()
                && let Some(result) = fold_unary_op(operator, v.clone())
            {
                *node = Node::Value(result);
                return true;
            }
        }
        _ => {}
    }
    false
}

fn fold_binary_op(op: &Operator, left: Value, right: Value) -> Option<Value> {
    use Value::*;
    match op {
        Operator::Add => match (&left, &right) {
            (Number(l), Number(r)) => Some(Number(l + r)),
            (Float(l), Float(r)) => Some(Float(l + r)),
            (String(l), String(r)) => Some(String(format!("{l}{r}"))),
            _ => None,
        },
        Operator::Subtract => match (&left, &right) {
            (Number(l), Number(r)) => Some(Number(l - r)),
            (Float(l), Float(r)) => Some(Float(l - r)),
            _ => None,
        },
        Operator::Multiply => match (&left, &right) {
            (Number(l), Number(r)) => Some(Number(l * r)),
            (Float(l), Float(r)) => Some(Float(l * r)),
            _ => None,
        },
        Operator::Divide => match (&left, &right) {
            (Number(l), Number(r)) if *r != 0 => Some(Number(l / r)),
            (Float(l), Float(r)) if *r != 0.0 => Some(Float(l / r)),
            _ => None,
        },
        Operator::Modulo => match (&left, &right) {
            (Number(l), Number(r)) if *r != 0 => Some(Number(l % r)),
            _ => None,
        },
        Operator::Pow => match (&left, &right) {
            (Number(l), Number(r)) => Some(Number(l.pow(*r as u32))),
            (Float(l), Float(r)) => Some(Float(l.powf(*r))),
            _ => None,
        },
        Operator::Equal => Some(Bool(left == right)),
        Operator::NotEqual => Some(Bool(left != right)),
        Operator::GreaterThan => {
            compare(&left, &right).map(|o| Bool(o == std::cmp::Ordering::Greater))
        }
        Operator::GreaterThanOrEqual => {
            compare(&left, &right).map(|o| Bool(o != std::cmp::Ordering::Less))
        }
        Operator::LessThan => compare(&left, &right).map(|o| Bool(o == std::cmp::Ordering::Less)),
        Operator::LessThanOrEqual => {
            compare(&left, &right).map(|o| Bool(o != std::cmp::Ordering::Greater))
        }
        Operator::And => Some(Bool(
            left.as_bool() == Some(true) && right.as_bool() == Some(true),
        )),
        Operator::Or => Some(Bool(
            left.as_bool() == Some(true) || right.as_bool() == Some(true),
        )),
        Operator::In => match (&left, &right) {
            (String(s), Map(m)) => Some(Bool(m.contains_key(&MapKey::String(s.clone())))),
            (Number(n), Map(m)) => Some(Bool(m.contains_key(&MapKey::Number(*n)))),
            (item, Array(arr)) => Some(Bool(arr.contains(item))),
            _ => None,
        },
        Operator::Contains => match (&left, &right) {
            (String(haystack), String(needle)) => Some(Bool(haystack.contains(needle.as_str()))),
            (Array(arr), item) => Some(Bool(arr.contains(item))),
            (Map(m), String(key)) => Some(Bool(m.contains_key(&MapKey::String(key.clone())))),
            _ => None,
        },
        Operator::StartsWith => match (&left, &right) {
            (String(s), String(prefix)) => Some(Bool(s.starts_with(prefix.as_str()))),
            _ => None,
        },
        Operator::EndsWith => match (&left, &right) {
            (String(s), String(suffix)) => Some(Bool(s.ends_with(suffix.as_str()))),
            _ => None,
        },
        Operator::Matches => match (&left, &right) {
            (String(s), String(pattern)) => regex::Regex::new(pattern.as_str())
                .ok()
                .map(|re| Bool(re.is_match(s.as_str()))),
            _ => None,
        },
    }
}

fn fold_unary_op(op: &UnaryOperator, val: Value) -> Option<Value> {
    use Value::*;
    match op {
        UnaryOperator::Not => match val {
            Bool(b) => Some(Bool(!b)),
            _ => None,
        },
        UnaryOperator::Positive => match val {
            Number(_) | Float(_) => Some(val),
            _ => None,
        },
        UnaryOperator::Negative => match val {
            Number(n) => Some(Number(-n)),
            Float(f) => Some(Float(-f)),
            _ => None,
        },
    }
}

fn compare(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    use Value::*;
    match (left, right) {
        (Number(l), Number(r)) => Some(l.cmp(r)),
        (Float(l), Float(r)) => l.partial_cmp(r),
        (String(l), String(r)) => Some(l.cmp(r)),
        _ => None,
    }
}

// ---- Boolean Algebra Minimization ----

fn try_boolean_minimization(node: &mut Node) -> bool {
    match node {
        Node::Unary {
            operator: UnaryOperator::Not,
            node: inner,
        } => match inner.as_ref() {
            Node::Value(Value::Bool(true)) => {
                *node = Node::Value(Value::Bool(false));
                return true;
            }
            Node::Value(Value::Bool(false)) => {
                *node = Node::Value(Value::Bool(true));
                return true;
            }
            Node::Unary {
                operator: UnaryOperator::Not,
                node: double_inner,
            } => {
                if let Node::Value(Value::Bool(_)) = double_inner.as_ref() {
                    *node = *double_inner.clone();
                    return true;
                }
            }
            _ => {}
        },
        Node::Operation {
            operator,
            left,
            right,
        } => match operator {
            Operator::And => {
                if let (Node::Value(Value::Bool(false)), _) = (left.as_ref(), right.as_ref()) {
                    *node = Node::Value(Value::Bool(false));
                    return true;
                }
            }
            Operator::Or => {
                if let (Node::Value(Value::Bool(true)), _) = (left.as_ref(), right.as_ref()) {
                    *node = Node::Value(Value::Bool(true));
                    return true;
                }
            }
            _ => {}
        },
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{bool_val, check_both_error, num, optimize_node};
    use crate::ast::node::Node;
    use crate::ast::operator::Operator;
    use crate::ast::unary_operator::UnaryOperator;
    use crate::{Context, Result, eval};

    // ---- Constant Folding (E2E) ----

    #[test]
    fn fold_int_arithmetic() -> Result<()> {
        assert_eq!(eval("3 * 4", &Context::default())?.to_string(), "12");
        assert_eq!(eval("3 + 4", &Context::default())?.to_string(), "7");
        assert_eq!(eval("10 - 3", &Context::default())?.to_string(), "7");
        assert_eq!(eval("7 / 2", &Context::default())?.to_string(), "3");
        assert_eq!(eval("7 % 3", &Context::default())?.to_string(), "1");
        assert_eq!(eval("2 ** 8", &Context::default())?.to_string(), "256");
        Ok(())
    }

    #[test]
    fn fold_float_arithmetic() -> Result<()> {
        assert_eq!(eval("2.5 + 3.5", &Context::default())?.to_string(), "6");
        assert_eq!(eval("3.0 * 4.0", &Context::default())?.to_string(), "12");
        assert_eq!(eval("6.0 / 2.0", &Context::default())?.to_string(), "3");
        Ok(())
    }

    #[test]
    fn fold_string_ops() -> Result<()> {
        assert_eq!(
            eval(r#""foo" + "bar""#, &Context::default())?.to_string(),
            r#""foobar""#
        );
        assert_eq!(
            eval(r#""foo" contains "o""#, &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval(r#""foo" startsWith "f""#, &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval(r#""foo" endsWith "o""#, &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval(r#""foo" matches "^f""#, &Context::default())?.to_string(),
            "true"
        );
        Ok(())
    }

    #[test]
    fn fold_comparisons() -> Result<()> {
        assert_eq!(eval("1 == 1", &Context::default())?.to_string(), "true");
        assert_eq!(eval("1 != 1", &Context::default())?.to_string(), "false");
        assert_eq!(eval("3 > 2", &Context::default())?.to_string(), "true");
        assert_eq!(eval("2 >= 2", &Context::default())?.to_string(), "true");
        assert_eq!(eval("1 < 2", &Context::default())?.to_string(), "true");
        assert_eq!(eval("2 <= 2", &Context::default())?.to_string(), "true");
        Ok(())
    }

    #[test]
    fn fold_in_contains() -> Result<()> {
        assert_eq!(
            eval(r#"1 in [1, 2, 3]"#, &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval(r#"4 in [1, 2, 3]"#, &Context::default())?.to_string(),
            "false"
        );
        assert_eq!(
            eval(r#""x" in {x: 1}"#, &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval(r#"["a", "b"] contains "a""#, &Context::default())?.to_string(),
            "true"
        );
        Ok(())
    }

    #[test]
    fn fold_unary() -> Result<()> {
        assert_eq!(eval("!true", &Context::default())?.to_string(), "false");
        assert_eq!(eval("!false", &Context::default())?.to_string(), "true");
        assert_eq!(eval("-5", &Context::default())?.to_string(), "-5");
        assert_eq!(eval("+3", &Context::default())?.to_string(), "3");
        Ok(())
    }

    #[test]
    fn fold_nested() -> Result<()> {
        assert_eq!(eval("(1 + 2) * 3", &Context::default())?.to_string(), "9");
        assert_eq!(eval("2 ** 3 + 1", &Context::default())?.to_string(), "9");
        Ok(())
    }

    // ---- Boolean Minimization ----

    #[test]
    fn boolean_short_circuit_and() -> Result<()> {
        assert_eq!(
            eval("true && true", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("true && false", &Context::default())?.to_string(),
            "false"
        );
        assert_eq!(
            eval("false && true", &Context::default())?.to_string(),
            "false"
        );
        assert_eq!(
            eval("false && false", &Context::default())?.to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn boolean_short_circuit_or() -> Result<()> {
        assert_eq!(
            eval("true || true", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("true || false", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("false || true", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("false || false", &Context::default())?.to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn boolean_double_negation_e2e() -> Result<()> {
        assert_eq!(
            eval("not (not true)", &Context::default())?.to_string(),
            "true"
        );
        assert_eq!(
            eval("not (not false)", &Context::default())?.to_string(),
            "false"
        );
        Ok(())
    }

    // ---- Algebraic Identities ----

    #[test]
    fn identity_add_zero() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x + 0", &ctx)?.to_string(), "5");
        assert_eq!(eval("0 + x", &ctx)?.to_string(), "5");
        Ok(())
    }

    #[test]
    fn identity_subtract_zero() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x - 0", &ctx)?.to_string(), "5");
        Ok(())
    }

    #[test]
    fn identity_multiply_one() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x * 1", &ctx)?.to_string(), "5");
        assert_eq!(eval("1 * x", &ctx)?.to_string(), "5");
        Ok(())
    }

    #[test]
    fn identity_multiply_zero() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x * 0", &ctx)?.to_string(), "0");
        assert_eq!(eval("0 * x", &ctx)?.to_string(), "0");
        Ok(())
    }

    #[test]
    fn identity_divide_one() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x / 1", &ctx)?.to_string(), "5");
        Ok(())
    }

    #[test]
    fn identity_divide_zero_numerator() -> Result<()> {
        assert_eq!(eval("0 / 5", &Context::default())?.to_string(), "0");
        Ok(())
    }

    #[test]
    fn identity_pow_zero() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x ** 0", &ctx)?.to_string(), "1");
        Ok(())
    }

    #[test]
    fn identity_pow_one() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("x ** 1", &ctx)?.to_string(), "5");
        assert_eq!(eval("1 ** x", &ctx)?.to_string(), "1");
        Ok(())
    }

    // ---- AST unit tests: verify fold/identity/boolean directly ----

    #[test]
    fn ast_fold_add() {
        let mut n = Node::Operation {
            operator: Operator::Add,
            left: Box::new(num(2)),
            right: Box::new(num(3)),
        };
        assert_eq!(optimize_node(&mut n), num(5));
    }

    #[test]
    fn ast_fold_multiply() {
        let mut n = Node::Operation {
            operator: Operator::Multiply,
            left: Box::new(num(3)),
            right: Box::new(num(4)),
        };
        assert_eq!(optimize_node(&mut n), num(12));
    }

    #[test]
    fn ast_identity_multiply_by_zero() {
        let mut n = Node::Operation {
            operator: Operator::Multiply,
            left: Box::new(Node::Ident("x".into())),
            right: Box::new(num(0)),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(n, original);
    }

    #[test]
    fn ast_identity_multiply_by_one_preserved() {
        let mut n = Node::Operation {
            operator: Operator::Multiply,
            left: Box::new(Node::Ident("x".into())),
            right: Box::new(num(1)),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(
            n, original,
            "x * 1 must not be simplified - type-unsafe for non-numeric x"
        );
    }

    #[test]
    fn ast_identity_add_zero_preserved() {
        let mut n = Node::Operation {
            operator: Operator::Add,
            left: Box::new(Node::Ident("x".into())),
            right: Box::new(num(0)),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(
            n, original,
            "x + 0 must not be simplified - type-unsafe for non-numeric x"
        );
    }

    #[test]
    fn ast_double_negation_ident_preserved() {
        let mut n = Node::Unary {
            operator: UnaryOperator::Not,
            node: Box::new(Node::Unary {
                operator: UnaryOperator::Not,
                node: Box::new(Node::Ident("x".into())),
            }),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(
            n, original,
            "!!x must not be simplified - type-unsafe for non-Bool x"
        );
    }

    #[test]
    fn ast_double_negation_bool_still_folds() {
        // !!true -> true via constant folding (not via !!x elimination)
        let mut n = Node::Unary {
            operator: UnaryOperator::Not,
            node: Box::new(Node::Unary {
                operator: UnaryOperator::Not,
                node: Box::new(bool_val(true)),
            }),
        };
        assert_eq!(optimize_node(&mut n), bool_val(true));
    }

    #[test]
    fn ast_boolean_not_true() {
        let mut n = Node::Unary {
            operator: UnaryOperator::Not,
            node: Box::new(bool_val(true)),
        };
        assert_eq!(optimize_node(&mut n), bool_val(false));
    }

    #[test]
    fn ast_boolean_and_with_true_left() {
        let mut n = Node::Operation {
            operator: Operator::And,
            left: Box::new(bool_val(true)),
            right: Box::new(Node::Ident("x".into())),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(n, original);
    }

    #[test]
    fn ast_boolean_and_with_false_left() {
        let mut n = Node::Operation {
            operator: Operator::And,
            left: Box::new(bool_val(false)),
            right: Box::new(Node::Ident("x".into())),
        };
        assert_eq!(optimize_node(&mut n), bool_val(false));
    }

    #[test]
    fn ast_boolean_or_with_true_left() {
        let mut n = Node::Operation {
            operator: Operator::Or,
            left: Box::new(bool_val(true)),
            right: Box::new(Node::Ident("x".into())),
        };
        assert_eq!(optimize_node(&mut n), bool_val(true));
    }

    #[test]
    fn ast_boolean_or_with_false_left() {
        let mut n = Node::Operation {
            operator: Operator::Or,
            left: Box::new(bool_val(false)),
            right: Box::new(Node::Ident("x".into())),
        };
        let original = n.clone();
        optimize_node(&mut n);
        assert_eq!(n, original);
    }

    // ---- Regression: algebraic fold preserves error semantics ----

    #[test]
    fn regr_fold_multiply_zero_errors() {
        check_both_error("missing * 0");
    }

    #[test]
    fn regr_fold_divide_zero_errors() {
        check_both_error("0 / missing");
    }

    #[test]
    fn regr_fold_pow_zero_errors() {
        check_both_error("missing ^ 0");
    }

    #[test]
    fn regr_fold_and_false_errors() {
        check_both_error("missing && false");
    }

    #[test]
    fn regr_fold_or_true_errors() {
        check_both_error("missing || true");
    }
}
