use crate::Value;
use crate::ast::node::Node;

pub fn optimize(node: &mut Node) -> bool {
    if let Node::Range(start, end) = node
        && let (Node::Value(Value::Number(s)), Node::Value(Value::Number(e))) =
            (start.as_ref(), end.as_ref())
    {
        *node = Node::Value(Value::Array((*s..=*e).map(Value::Number).collect()));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::num;
    use crate::ast::node::Node;
    use crate::{Context, Result, Value, eval};

    #[test]
    fn range_expansion() -> Result<()> {
        assert_eq!(eval("0..2", &Context::default())?.to_string(), "[0, 1, 2]");
        assert_eq!(eval("5..5", &Context::default())?.to_string(), "[5]");
        Ok(())
    }

    #[test]
    fn ast_range_expansion() {
        let mut n = Node::Range(Box::new(num(0)), Box::new(num(2)));
        n.expand_ranges();
        let expected = Node::Value(Value::Array(vec![
            Value::Number(0),
            Value::Number(1),
            Value::Number(2),
        ]));
        assert_eq!(n, expected);
    }
}
