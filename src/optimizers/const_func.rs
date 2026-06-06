use indexmap::IndexMap;

use crate::ast::node::Node;
use crate::functions::FunctionDefinition;

pub fn optimize(node: &mut Node, functions: &IndexMap<String, FunctionDefinition<'_>>) -> bool {
    if let Node::Func {
        ident,
        args,
        predicate,
        threshold,
        throws,
        map_node,
    } = node
    {
        let const_eval = match functions.get(ident.as_str()) {
            Some(def) => match def.metadata.const_eval {
                Some(f) => f,
                None => return false,
            },
            None => return false,
        };
        if predicate.is_some() || *throws || map_node.is_some() || threshold.is_some() {
            return false;
        }
        let const_args: Option<Vec<_>> = args.iter().map(|a| a.literal_value()).collect();
        if let Some(arg_vals) = const_args
            && let Some(result) = const_eval(&arg_vals)
        {
            *node = Node::Value(result);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::optimize;
    use crate::ast::node::Node;
    use crate::{Environment, MapKey, Value};

    fn env() -> Environment<'static> {
        Environment::new()
    }

    fn fold(node: &mut Node) -> bool {
        let e = env();
        optimize(node, &e.functions)
    }

    fn num(n: i64) -> Node {
        Node::Value(Value::Number(n))
    }

    fn str_val(s: &str) -> Node {
        Node::Value(Value::String(s.to_string()))
    }

    fn arr(items: Vec<Node>) -> Node {
        Node::Array(items)
    }

    fn func(name: &str, args: Vec<Node>) -> Node {
        Node::Func {
            ident: name.to_string(),
            args,
            predicate: None,
            threshold: None,
            throws: false,
            map_node: None,
        }
    }

    // ---- len ----

    #[test]
    fn folds_len_on_array() {
        let mut n = func("len", vec![arr(vec![num(1), num(2), num(3)])]);
        assert!(fold(&mut n));
        assert_eq!(n, num(3));
    }

    #[test]
    fn folds_len_on_string() {
        let mut n = func("len", vec![str_val("hello")]);
        assert!(fold(&mut n));
        assert_eq!(n, num(5));
    }

    #[test]
    fn folds_len_on_map() {
        let mut n = func(
            "len",
            vec![Node::Value(Value::Map(
                [
                    (MapKey::from("a"), num(1).into_val()),
                    (MapKey::from("b"), num(2).into_val()),
                ]
                .into(),
            ))],
        );
        assert!(fold(&mut n));
        assert_eq!(n, num(2));
    }

    #[test]
    fn does_not_fold_len_on_ident() {
        let mut n = func("len", vec![Node::Ident("x".into())]);
        assert!(!fold(&mut n));
    }

    #[test]
    fn does_not_fold_len_on_bool() {
        let mut n = func("len", vec![Node::Value(Value::Bool(true))]);
        assert!(!fold(&mut n));
    }

    // ---- first / last ----

    #[test]
    fn folds_first() {
        let mut n = func("first", vec![arr(vec![num(10), num(20)])]);
        assert!(fold(&mut n));
        assert_eq!(n, num(10));
    }

    #[test]
    fn folds_first_empty() {
        let mut n = func("first", vec![arr(vec![])]);
        assert!(fold(&mut n));
        assert_eq!(n, Node::Value(Value::Nil));
    }

    #[test]
    fn folds_last() {
        let mut n = func("last", vec![arr(vec![num(10), num(20)])]);
        assert!(fold(&mut n));
        assert_eq!(n, num(20));
    }

    // ---- upper / lower ----

    #[test]
    fn folds_upper() {
        let mut n = func("upper", vec![str_val("hello")]);
        assert!(fold(&mut n));
        assert_eq!(n, str_val("HELLO"));
    }

    #[test]
    fn folds_lower() {
        let mut n = func("lower", vec![str_val("HELLO")]);
        assert!(fold(&mut n));
        assert_eq!(n, str_val("hello"));
    }

    // ---- trim ----

    #[test]
    fn folds_trim_no_chars() {
        let mut n = func("trim", vec![str_val("  hello  ")]);
        assert!(fold(&mut n));
        assert_eq!(n, str_val("hello"));
    }

    #[test]
    fn folds_trim_with_chars() {
        let mut n = func("trim", vec![str_val("--hello--"), str_val("-")]);
        assert!(fold(&mut n));
        assert_eq!(n, str_val("hello"));
    }

    #[test]
    fn does_not_fold_trim_wrong_type() {
        let mut n = func("trim", vec![num(42)]);
        assert!(!fold(&mut n));
    }

    // ---- trimPrefix / trimSuffix ----

    #[test]
    fn folds_trim_prefix() {
        let mut n = func(
            "trimPrefix",
            vec![str_val("hello world"), str_val("hello ")],
        );
        assert!(fold(&mut n));
        assert_eq!(n, str_val("world"));
    }

    #[test]
    fn folds_trim_suffix() {
        let mut n = func(
            "trimSuffix",
            vec![str_val("hello world"), str_val(" world")],
        );
        assert!(fold(&mut n));
        assert_eq!(n, str_val("hello"));
    }

    // ---- keys / values ----

    #[test]
    fn folds_keys() {
        let m = Node::Value(Value::Map(
            [
                (MapKey::from("x"), num(1).into_val()),
                (MapKey::from("y"), num(2).into_val()),
            ]
            .into(),
        ));
        let mut n = func("keys", vec![m]);
        assert!(fold(&mut n));
        assert_eq!(
            n,
            Node::Value(Value::Array(vec![
                Value::String("x".into()),
                Value::String("y".into())
            ]))
        );
    }

    #[test]
    fn folds_values() {
        let m = Node::Value(Value::Map([(MapKey::from("x"), num(1).into_val())].into()));
        let mut n = func("values", vec![m]);
        assert!(fold(&mut n));
        assert_eq!(n, Node::Value(Value::Array(vec![Value::Number(1)])));
    }

    // ---- replace ----

    #[test]
    fn folds_replace() {
        let mut n = func("replace", vec![str_val("abac"), str_val("a"), str_val("x")]);
        assert!(fold(&mut n));
        assert_eq!(n, str_val("xbxc"));
    }

    // ---- repeat ----

    #[test]
    fn folds_repeat() {
        let mut n = func("repeat", vec![str_val("ab"), num(2)]);
        assert!(fold(&mut n));
        assert_eq!(n, str_val("ababab"));
    }

    // ---- indexOf / lastIndexOf ----

    #[test]
    fn folds_index_of_found() {
        let mut n = func("indexOf", vec![str_val("hello"), str_val("e")]);
        assert!(fold(&mut n));
        assert_eq!(n, num(1));
    }

    #[test]
    fn folds_index_of_not_found() {
        let mut n = func("indexOf", vec![str_val("hello"), str_val("z")]);
        assert!(fold(&mut n));
        assert_eq!(n, num(-1));
    }

    #[test]
    fn folds_last_index_of() {
        let mut n = func("lastIndexOf", vec![str_val("banana"), str_val("a")]);
        assert!(fold(&mut n));
        assert_eq!(n, num(5));
    }

    // ---- hasPrefix / hasSuffix ----

    #[test]
    fn folds_has_prefix_true() {
        let mut n = func("hasPrefix", vec![str_val("hello"), str_val("he")]);
        assert!(fold(&mut n));
        assert_eq!(n, Node::Value(Value::Bool(true)));
    }

    #[test]
    fn folds_has_prefix_false() {
        let mut n = func("hasPrefix", vec![str_val("hello"), str_val("xx")]);
        assert!(fold(&mut n));
        assert_eq!(n, Node::Value(Value::Bool(false)));
    }

    #[test]
    fn folds_has_suffix() {
        let mut n = func("hasSuffix", vec![str_val("hello"), str_val("lo")]);
        assert!(fold(&mut n));
        assert_eq!(n, Node::Value(Value::Bool(true)));
    }

    // ---- Rejection: predicate / throws / threshold / map_node ----

    #[test]
    fn does_not_fold_with_predicate() {
        let mut n = Node::Func {
            ident: "len".into(),
            args: vec![arr(vec![num(1)])],
            predicate: Some(Box::new(Default::default())),
            threshold: None,
            throws: false,
            map_node: None,
        };
        assert!(!fold(&mut n));
    }

    #[test]
    fn does_not_fold_with_throws() {
        let mut n = Node::Func {
            ident: "len".into(),
            args: vec![arr(vec![num(1)])],
            predicate: None,
            threshold: None,
            throws: true,
            map_node: None,
        };
        assert!(!fold(&mut n));
    }

    #[test]
    fn does_not_fold_with_threshold() {
        let mut n = Node::Func {
            ident: "len".into(),
            args: vec![arr(vec![num(1)])],
            predicate: None,
            threshold: Some(10),
            throws: false,
            map_node: None,
        };
        assert!(!fold(&mut n));
    }

    #[test]
    fn does_not_fold_with_map_node() {
        let mut n = Node::Func {
            ident: "len".into(),
            args: vec![arr(vec![num(1)])],
            predicate: None,
            threshold: None,
            throws: false,
            map_node: Some(Box::new(num(1))),
        };
        assert!(!fold(&mut n));
    }

    // ---- Non-pure function (no const_eval) ----

    #[test]
    fn does_not_fold_non_pure_function() {
        let mut n = func("split", vec![str_val("a,b"), str_val(",")]);
        assert!(!fold(&mut n));
    }

    // ---- Unknown function ----

    #[test]
    fn does_not_fold_unknown_function() {
        let mut n = func("nonexistent", vec![num(1)]);
        assert!(!fold(&mut n));
    }

    // ---- E2E via Environment::compile ----

    #[test]
    fn e2e_folds_len_in_unreferenced_binding() {
        let env = Environment::new();
        let program = env.compile("let c = len([1, 2]); 1").unwrap();
        assert!(!program.lines().iter().any(|(id, _)| id == "c"));
    }

    #[test]
    fn e2e_folds_len_after_propagation() {
        let env = Environment::new();
        let program = env.compile("let a = [1, 2, 3]; let b = len(a); 1").unwrap();
        assert!(program.lines().is_empty());
    }

    #[test]
    fn e2e_preserves_len_with_ident_error() {
        let e = Environment::new();
        assert!(
            e.eval("let unused = len(missing); 1", &Default::default())
                .is_err()
        );
    }

    #[test]
    fn e2e_custom_len_not_folded() {
        let mut env = Environment::new();
        env.add_function("len", |c| Ok(Value::Number(c.args.len() as i64 * 100)));
        let program = env.compile("let c = len([1, 2]); 1").unwrap();
        assert!(program.lines().iter().any(|(id, _)| id == "c"));
    }

    #[test]
    fn e2e_folds_multiple_pure_calls() {
        let env = Environment::new();
        let program = env
            .compile("let a = len([1, 2, 3]); let b = len(\"hello\"); 1")
            .unwrap();
        assert!(program.lines().is_empty());
    }

    // ---- Helpers for Node -> Value ----

    trait IntoVal {
        fn into_val(self) -> Value;
    }

    impl IntoVal for Node {
        fn into_val(self) -> Value {
            match self {
                Node::Value(v) => v,
                _ => panic!("not a value node"),
            }
        }
    }
}
