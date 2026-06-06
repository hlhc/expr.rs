use crate::ast::node::Node;
use crate::ast::operator::Operator;
use crate::ast::program::Program;

/// Combines multiple predicate calls on the same array into a single call.
///   all(x, p1) && all(x, p2) → all(x, p1 && p2)
///   any(x, p1) || any(x, p2) → any(x, p1 || p2)
///   none(x, p1) && none(x, p2) → none(x, p1 || p2)
pub fn optimize(node: &mut Node) -> bool {
    let Node::Operation {
        operator,
        left,
        right,
    } = node
    else {
        return false;
    };

    let Node::Func {
        ident: left_ident,
        args: left_args,
        predicate: left_pred,
        ..
    } = left.as_ref()
    else {
        return false;
    };

    let Node::Func {
        ident: right_ident,
        args: right_args,
        predicate: right_pred,
        ..
    } = right.as_ref()
    else {
        return false;
    };

    if left_ident != right_ident {
        return false;
    }
    if left_args.len() != 1 || right_args.len() != 1 {
        return false;
    }
    // Check that both operate on the same array (by structural equality of args[0])
    if left_args[0] != right_args[0] {
        return false;
    }

    let left_pred = left_pred.as_ref();
    let right_pred = right_pred.as_ref();
    let (Some(lp), Some(rp)) = (left_pred, right_pred) else {
        return false;
    };

    let combined_op = combined_operator(left_ident.as_str(), operator);
    let Some(co) = combined_op else {
        return false;
    };

    let combined_pred = Node::Operation {
        operator: co.clone(),
        left: Box::new(lp.expr.clone()),
        right: Box::new(rp.expr.clone()),
    };

    *node = Node::Func {
        ident: left_ident.clone(),
        args: vec![left_args[0].clone()],
        predicate: Some(Box::new(Program {
            lines: Vec::new(),
            expr: combined_pred,
        })),
        threshold: None,
        throws: false,
        map_node: None,
    };
    true
}

fn combined_operator(fn_name: &str, op: &Operator) -> Option<Operator> {
    match fn_name {
        "all" => matches!(op, Operator::And).then_some(Operator::And),
        "any" => matches!(op, Operator::Or).then_some(Operator::Or),
        "none" => matches!(op, Operator::And).then_some(Operator::Or),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Result, eval};

    #[test]
    fn predicate_combination_all_and_all() -> Result<()> {
        assert_eq!(
            eval(
                "all([1, 2, 3], {# > 0}) && all([1, 2, 3], {# < 4})",
                &Context::default()
            )?
            .to_string(),
            "true"
        );
        assert_eq!(
            eval(
                "all([1, 2, 3], {# > 2}) && all([1, 2, 3], {# < 4})",
                &Context::default()
            )?
            .to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn predicate_combination_any_or_any() -> Result<()> {
        assert_eq!(
            eval(
                "any([1, 2, 3], {# > 2}) || any([1, 2, 3], {# < 0})",
                &Context::default()
            )?
            .to_string(),
            "true"
        );
        assert_eq!(
            eval(
                "any([1, 2, 3], {# > 5}) || any([1, 2, 3], {# < 0})",
                &Context::default()
            )?
            .to_string(),
            "false"
        );
        Ok(())
    }

    #[test]
    fn predicate_combination_none_and_none() -> Result<()> {
        assert_eq!(
            eval(
                "none([1, 2, 3], {# > 3}) && none([1, 2, 3], {# < 1})",
                &Context::default()
            )?
            .to_string(),
            "true"
        );
        Ok(())
    }
}
