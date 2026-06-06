mod fold;
mod prop;
mod dce;
mod pipe;
mod range;
mod ternary;
mod nil_coalesce;
mod const_key;
mod in_array;
mod in_range;
mod filter_first;
mod filter_last;
mod filter_len;
mod filter_map;
mod predicate_combination;
mod sum_array;
mod sum_map;
mod sum_range;
mod count_any;
mod count_threshold;

use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;
use crate::ast::program::Program;

impl PostfixOperator {
    fn optimize_children(&mut self) {
        match self {
            PostfixOperator::Index { idx, .. } => idx.optimize(),
            PostfixOperator::Default(n) | PostfixOperator::Pipe(n) => n.optimize(),
            PostfixOperator::Ternary { left, right } => {
                left.optimize();
                right.optimize();
            }
            PostfixOperator::Range(..) => {}
        }
    }
}

impl Node {
    pub fn optimize(&mut self) {
        self.optimize_children();
        self.apply_optimizations();
    }

    fn optimize_children(&mut self) {
        match self {
            Node::Array(items) => items.iter_mut().for_each(|i| i.optimize()),
            Node::Range(start, end) => {
                start.optimize();
                end.optimize();
            }
            Node::Func { args, map_node, .. } => {
                args.iter_mut().for_each(|a| a.optimize());
                if let Some(mn) = map_node {
                    mn.optimize();
                }
            }
            Node::Unary { node, .. } => node.optimize(),
            Node::Operation { left, right, .. } => {
                left.optimize();
                right.optimize();
            }
            Node::Postfix { node, operator } => {
                node.optimize();
                operator.optimize_children();
            }
            Node::Value(_) | Node::Ident(_) => {}
        }
    }

    fn apply_optimizations(&mut self) {
        // Restructuring passes — these may change the node variant (e.g.
        // Func → Operation, Operation → Func, Func → Value). After each
        // successful rewrite, re-optimize children so they are fully
        // simplified before fold runs on the restructured node.
        let mut restructured = false;
        match self {
            Node::Operation { .. } => {
                restructured |= in_array::optimize(self);
                restructured |= in_range::optimize(self);
                restructured |= predicate_combination::optimize(self);
            }
            Node::Func { .. } => {
                restructured |= sum_array::optimize(self);
                restructured |= sum_range::optimize(self);
            }
            _ => {}
        }
        if restructured {
            self.optimize_children();
        }

        // Fold loop — only runs on Operation and Unary (the only variants
        // fold can modify). Also runs unconditionally if restructuring
        // changed the node (Func → Operation, etc.).
        if restructured || matches!(self, Node::Operation { .. } | Node::Unary { .. }) {
            for _ in 0..100 {
                if !fold::optimize(self) {
                    break;
                }
            }
        }

        // Single-pass transformations — dispatch by variant since these
        // never change the node into a form that another pass needs to see.
        match self {
            Node::Postfix { .. } => {
                ternary::optimize(self);
                nil_coalesce::optimize(self);
                pipe::optimize(self);
                const_key::optimize(self);
                filter_first::optimize(self);
                filter_last::optimize(self);
            }
            Node::Func { .. } => {
                filter_map::optimize(self);
                filter_len::optimize(self);
                sum_map::optimize(self);
                filter_first::optimize(self);
                filter_last::optimize(self);
            }
            Node::Operation { .. } => {
                count_any::optimize(self);
                count_threshold::optimize(self);
            }
            _ => {}
        }
    }

    /// Expand remaining range nodes to arrays. Must run AFTER all structural
    /// optimizations (especially in_range) so that `x in m..n` is converted
    /// before m..n is expanded.
    pub fn expand_ranges(&mut self) {
        self.expand_ranges_children();
        range::optimize(self);
    }

    fn expand_ranges_children(&mut self) {
        match self {
            Node::Array(items) => items.iter_mut().for_each(|i| i.expand_ranges()),
            Node::Range(start, end) => {
                start.expand_ranges();
                end.expand_ranges();
            }
            Node::Func { args, map_node, .. } => {
                args.iter_mut().for_each(|a| a.expand_ranges());
                if let Some(mn) = map_node {
                    mn.expand_ranges();
                }
            }
            Node::Unary { node, .. } => node.expand_ranges(),
            Node::Operation { left, right, .. } => {
                left.expand_ranges();
                right.expand_ranges();
            }
            Node::Postfix { node, operator } => {
                node.expand_ranges();
                match operator {
                    PostfixOperator::Index { idx, .. } => idx.expand_ranges(),
                    PostfixOperator::Default(n) | PostfixOperator::Pipe(n) => n.expand_ranges(),
                    PostfixOperator::Ternary { left, right } => {
                        left.expand_ranges();
                        right.expand_ranges();
                    }
                    PostfixOperator::Range(..) => {}
                }
            }
            Node::Value(_) | Node::Ident(_) => {}
        }
    }
}

impl Program {
    pub fn optimize(&mut self) {
        // Phase 1: bottom-up structural transformations (fold, in_range, etc.)
        for (_, value) in &mut self.lines {
            value.optimize();
        }
        self.expr.optimize();

        // Phase 2: expand ranges BEFORE prop so const_key can resolve r[1]
        // after constant substitution introduces new array literals.
        for (_, value) in &mut self.lines {
            value.expand_ranges();
        }
        self.expr.expand_ranges();

        // Phase 3–4: constant propagation. When constants are found and
        // substituted, prop re-optimizes and expands ranges only for the
        // binding(s) that actually changed (not the entire program).
        prop::optimize(self);

        // Phase 5: dead code elimination
        dce::optimize(self);
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::ast::node::Node;
    use crate::{Context, Value, compile_opts, run, Result};

    pub(crate) fn num(n: i64) -> Node {
        Node::Value(Value::Number(n))
    }

    pub(crate) fn bool_val(b: bool) -> Node {
        Node::Value(Value::Bool(b))
    }

    pub(crate) fn optimize_node(n: &mut Node) -> Node {
        n.optimize();
        n.clone()
    }

    pub(crate) fn check_optimized_eq_unoptimized(code: &str, expected: &str) -> Result<()> {
        let ctx = Context::default();
        let opt_program = compile_opts(code, true)?;
        let unopt_program = compile_opts(code, false)?;
        let opt_result = run(opt_program, &ctx)?;
        let unopt_result = run(unopt_program, &ctx)?;
        assert_eq!(
            format!("{opt_result}"),
            expected,
            "optimized result mismatch for: {code}"
        );
        assert_eq!(
            format!("{unopt_result}"),
            expected,
            "unoptimized result mismatch for: {code}"
        );
        Ok(())
    }

    pub(crate) fn check_both_error(code: &str) {
        let ctx = Context::default();
        for optimized in [true, false] {
            let label = if optimized { "optimized" } else { "unoptimized" };
            let program = compile_opts(code, optimized).unwrap();
            let result = run(program, &ctx);
            assert!(
                result.is_err(),
                "{label} should have errored for: {code}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result};

    #[test]
    fn combined_fold_and_propagation() -> Result<()> {
        assert_eq!(
            eval("let x = 2 + 3; x * 2", &Context::default())?.to_string(),
            "10"
        );
        Ok(())
    }

    #[test]
    fn combined_identity_and_fold() -> Result<()> {
        let ctx = Context::from_iter([("x", 5)]);
        assert_eq!(eval("(x + 0) * 2", &ctx)?.to_string(), "10");
        Ok(())
    }

    #[test]
    fn combined_ternary_and_fold() -> Result<()> {
        assert_eq!(
            eval("(true ? 3 : 4) * 2", &Context::default())?.to_string(),
            "6"
        );
        Ok(())
    }

    #[test]
    fn deep_nested_folding() -> Result<()> {
        assert_eq!(
            eval("((1 + 2) * (3 + 4)) / (5 - 3)", &Context::default())?.to_string(),
            "10"
        );
        Ok(())
    }
}
