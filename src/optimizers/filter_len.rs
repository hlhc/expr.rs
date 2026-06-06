use crate::ast::node::Node;

/// Converts `len(filter(arr, pred))` to `count(arr, pred)`.
/// count() can be more efficient because it doesn't need to build the filtered array.
pub fn optimize(node: &mut Node) -> bool {
    if let Node::Func { ident, args, .. } = node
        && ident == "len"
        && args.len() == 1
        && let Node::Func { ident: inner_ident, args: inner_args, predicate, .. } = &args[0]
        && inner_ident == "filter"
        && inner_args.len() == 1
    {
        *node = Node::Func {
            ident: "count".to_string(),
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
    use crate::{Context, eval, Result};
    use crate::ast::node::Node;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn filter_len_conversion() -> Result<()> {
        assert_eq!(
            eval("len(filter([1, 2, 3, 4], {# > 2}))", &Context::default())?.to_string(),
            "2"
        );
        Ok(())
    }

    #[test]
    fn ast_filter_len_converts_to_count() {
        let mut n = Node::Func {
            ident: "len".into(),
            args: vec![Node::Func {
                ident: "filter".into(),
                args: vec![Node::Array(vec![num(1), num(2)])],
                predicate: None,
                threshold: None,
                throws: false,
                map_node: None,
            }],
            predicate: None,
            threshold: None,
            throws: false,
            map_node: None,
        };
        let optimized = optimize_node(&mut n);
        assert!(matches!(optimized, Node::Func { ident, .. }
            if ident == "count"));
    }
}
