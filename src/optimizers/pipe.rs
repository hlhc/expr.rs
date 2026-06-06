use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;

pub fn optimize(node: &mut Node) -> bool {
    if let Node::Postfix {
        operator: PostfixOperator::Pipe(func_node),
        node: piped_value,
    } = node
        && let Node::Func {
            ident,
            args,
            predicate,
            ..
        } = func_node.as_mut()
    {
        let mut new_args = args.clone();
        new_args.push(*piped_value.clone());
        *node = Node::Func {
            ident: ident.clone(),
            args: new_args,
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
    use crate::ast::postfix_operator::PostfixOperator;
    use super::super::test_helpers::{num, optimize_node};

    #[test]
    fn pipe_unwrapping() -> Result<()> {
        assert_eq!(eval("[3, 1, 2] | sort()", &Context::default())?.to_string(), "[1, 2, 3]");
        assert_eq!(eval("[1, 2, 3] | len()", &Context::default())?.to_string(), "3");
        Ok(())
    }

    #[test]
    fn ast_pipe_converts_to_func_call() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Pipe(Box::new(Node::Func {
                ident: "len".into(),
                args: vec![],
                predicate: None,
                threshold: None,
                throws: false,
                map_node: None,
            })),
            node: Box::new(Node::Array(vec![num(1), num(2), num(3)])),
        };
        let optimized = optimize_node(&mut n);
        match &optimized {
            Node::Func { ident, args, .. } => {
                assert_eq!(ident, "len");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Node::Array(..)));
            }
            other => panic!("Expected Func node, got {other:?}"),
        }
    }

    #[test]
    fn ast_pipe_appends_to_existing_args() {
        let mut n = Node::Postfix {
            operator: PostfixOperator::Pipe(Box::new(Node::Func {
                ident: "filter".into(),
                args: vec![Node::Array(vec![num(1), num(2), num(3)])],
                predicate: None,
                threshold: None,
                throws: false,
                map_node: None,
            })),
            node: Box::new(num(1)),
        };
        let optimized = optimize_node(&mut n);
        match &optimized {
            Node::Func { ident, args, .. } => {
                assert_eq!(ident, "filter");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Node::Array(..)));
                assert_eq!(args[1], num(1));
            }
            other => panic!("Expected Func node, got {other:?}"),
        }
    }
}
