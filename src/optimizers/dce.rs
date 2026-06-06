use std::collections::HashSet;

use crate::Value;
use crate::ast::node::Node;
use crate::ast::program::Program;

pub fn optimize(program: &mut Program) {
    let mut live: HashSet<String> = program.collect_expr_idents();

    loop {
        let prev_len = live.len();
        for (id, value) in &program.lines {
            if live.contains(id.as_str()) {
                value.collect_idents(&mut live);
            }
        }
        if live.len() == prev_len {
            break;
        }
    }

    // Only elide unreferenced bindings whose values are inert (cannot produce
    // runtime errors). Pure Value literals and numeric Ranges are inert;
    // everything else (Idents, Func calls, Operations, Postfix, etc.) is kept.
    program
        .lines
        .retain(|(id, v)| live.contains(id.as_str()) || !is_inert(v));
}

fn is_inert(node: &Node) -> bool {
    match node {
        Node::Value(_) => true,
        Node::Array(items) => items.iter().all(is_inert),
        Node::Range(start, end) => matches!(
            (start.as_ref(), end.as_ref()),
            (Node::Value(Value::Number(_)), Node::Value(Value::Number(_)))
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use super::super::test_helpers::check_optimized_eq_unoptimized;
    use crate::{CompileOpts, Environment};
    use crate::{Result, compile};

    fn optimistic_compile(code: &str) -> Result<crate::Program> {
        Environment::new().compile_opts(code, &CompileOpts { optimized: true })
    }

    #[test]
    fn dce_removes_unreferenced_pure_value() -> Result<()> {
        let program = compile("let unused = 42; 1")?;
        assert!(!program.lines.iter().any(|(id, _)| id == "unused"));
        Ok(())
    }

    #[test]
    fn dce_keeps_unreferenced_func_call() -> Result<()> {
        let program = compile("let unused = len([1, 2]); 1")?;
        assert!(program.lines.iter().any(|(id, _)| id == "unused"));
        Ok(())
    }

    #[test]
    fn dce_keeps_unreferenced_ident() -> Result<()> {
        let program = compile("let unused = missing; 1")?;
        assert!(program.lines.iter().any(|(id, _)| id == "unused"));
        Ok(())
    }

    #[test]
    fn dce_removes_unreferenced_numeric_range() -> Result<()> {
        let program = compile("let unused = 0..2; 1")?;
        assert!(!program.lines.iter().any(|(id, _)| id == "unused"));
        Ok(())
    }

    #[test]
    fn dce_keeps_unreferenced_non_numeric_range() -> Result<()> {
        let program = compile("let unused = true..false; 1")?;
        assert!(program.lines.iter().any(|(id, _)| id == "unused"));
        Ok(())
    }

    #[test]
    fn regr_dce_unreferenced_ident_errors() {
        super::super::test_helpers::check_both_error("let unused = missing; 1");
        super::super::test_helpers::check_both_error(r#"let unused = "a" / 2; 2"#);
        super::super::test_helpers::check_both_error("let unused = true..false; 1");
    }

    #[test]
    fn dce_keeps_referenced_binding() -> Result<()> {
        let program = compile("let used = len([1, 2]); used")?;
        assert!(program.lines.iter().any(|(id, _)| id == "used"));
        Ok(())
    }

    #[test]
    fn dce_walks_transitive_references() -> Result<()> {
        let program = compile("let x = len([1]); let y = x; y")?;
        assert!(program.lines.iter().any(|(id, _)| id == "x"));
        assert!(program.lines.iter().any(|(id, _)| id == "y"));
        Ok(())
    }

    #[test]
    fn dce_fully_propagated_expr_removes_all() -> Result<()> {
        let program = compile("let x = 2 + 3; let y = x + 1; y")?;
        assert!(program.lines.is_empty());
        Ok(())
    }

    // ---- New: env-aware optimization removes const-folded unreferenced bindings ----

    #[test]
    fn dce_removes_unreferenced_folded_func_call() -> Result<()> {
        let program = optimistic_compile("let c = len([1, 2]); 1")?;
        assert!(!program.lines.iter().any(|(id, _)| id == "c"));
        Ok(())
    }

    #[test]
    fn dce_propagates_and_removes_folded_func_call() -> Result<()> {
        let program = optimistic_compile("let a = [1, 2, 3]; let b = len(a); 1")?;
        assert!(program.lines.is_empty());
        Ok(())
    }

    #[test]
    fn dce_keeps_live_folded_func_binding() -> Result<()> {
        let program = optimistic_compile("let used = len([1, 2, 3]); used")?;
        assert!(program.lines.is_empty());
        Ok(())
    }

    // ---- Regression: DCE preserves closure dependencies ----

    #[test]
    fn regr_dce_closure_deps() -> Result<()> {
        check_optimized_eq_unoptimized("let x = 1; map([1], {# + x})", "[2]")
    }

    #[test]
    fn regr_dce_live_func_call_binding() -> Result<()> {
        check_optimized_eq_unoptimized("let x = 1; let y = x + len([1]); y", "2")
    }
}
