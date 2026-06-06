use crate::Value;
use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;

/// Converts `filter(arr, pred)[0]` to `find(arr, pred)` for early termination.
/// Also converts `first(filter(arr, pred))` to `find(arr, pred)`.
pub fn optimize(node: &mut Node) -> bool {
    // Pattern 1: filter(arr, pred)[0]
    if let Node::Postfix {
        operator: PostfixOperator::Index {
            idx,
            optional: false,
        },
        node: container,
    } = node
        && let Node::Value(Value::Number(0)) = idx.as_ref()
        && let Node::Func {
            ident,
            args,
            predicate,
            ..
        } = container.as_ref()
        && ident == "filter"
        && args.len() == 1
    {
        *node = Node::Func {
            ident: "find".to_string(),
            args: args.clone(),
            predicate: predicate.clone(),
            threshold: None,
            throws: false,
            map_node: None,
        };
        return true;
    }
    // Pattern 2: first(filter(arr, pred))
    if let Node::Func { ident, args, .. } = node
        && ident == "first"
        && args.len() == 1
        && let Node::Func {
            ident: inner_ident,
            args: inner_args,
            predicate,
            ..
        } = &args[0]
        && inner_ident == "filter"
        && inner_args.len() == 1
    {
        *node = Node::Func {
            ident: "find".to_string(),
            args: inner_args.clone(),
            predicate: predicate.clone(),
            threshold: None,
            throws: false,
            map_node: None,
        };
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{check_optimized_eq_unoptimized, num, optimize_node};
    use crate::ast::node::Node;
    use crate::ast::postfix_operator::PostfixOperator;
    use crate::{Context, Result, eval};

    #[test]
    fn filter_first_optimization() -> Result<()> {
        assert_eq!(
            eval("filter([1, 2, 3], {# > 1})[0]", &Context::default())?.to_string(),
            "2"
        );
        Ok(())
    }

    #[test]
    fn filter_first_not_found_no_throw() -> Result<()> {
        assert_eq!(
            eval("first(filter([1, 2, 3], {# > 5}))", &Context::default())?.to_string(),
            "nil"
        );
        Ok(())
    }

    #[test]
    fn ast_filter_first_converts_to_find() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Index {
                idx: Box::new(num(0)),
                optional: false,
            },
            node: Box::new(Node::Func {
                ident: "filter".into(),
                args: vec![Node::Array(vec![num(1), num(2), num(3)])],
                predicate: None,
                threshold: None,
                throws: false,
                map_node: None,
            }),
        };
        let optimized = optimize_node(&mut n);
        assert!(matches!(optimized, Node::Func { ident, throws: false, .. }
            if ident == "find"));
    }

    // ---- Regression ----

    #[test]
    fn regr_filter_index_zero_no_match() -> Result<()> {
        check_optimized_eq_unoptimized("filter([1], {# > 2})[0]", "nil")
    }
}
