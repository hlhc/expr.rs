use crate::ast::node::Node;
use crate::ast::postfix_operator::PostfixOperator;
use crate::ast::program::Program;
use indexmap::IndexMap;

pub fn optimize(program: &mut Program) -> bool {
    let mut constants: IndexMap<String, Node> = IndexMap::new();

    for (id, value) in &program.lines {
        if value.is_constant() {
            constants.insert(id.clone(), value.clone());
        } else if references_any_const(value, &constants) {
            let resolved = resolve_ident_chain(value, &constants);
            if resolved.is_constant() {
                constants.insert(id.clone(), resolved);
            }
        }
    }

    if constants.is_empty() {
        return false;
    }

    let any_used = program
        .lines
        .iter()
        .any(|(_, v)| references_any_const(v, &constants))
        || references_any_const(&program.expr, &constants);

    if !any_used {
        return false;
    }

    // Substitute and re-optimize only the bindings that actually changed.
    let mut changed = false;
    for (_, value) in &mut program.lines {
        if substitute_constants(value, &constants) {
            value.optimize();
            value.expand_ranges();
            changed = true;
        }
    }
    if substitute_constants(&mut program.expr, &constants) {
        program.expr.optimize();
        program.expr.expand_ranges();
        changed = true;
    }

    changed
}

fn references_any_const(node: &Node, constants: &IndexMap<String, Node>) -> bool {
    match node {
        Node::Ident(id) => constants.contains_key(id.as_str()),
        Node::Operation { left, right, .. } => {
            references_any_const(left, constants) || references_any_const(right, constants)
        }
        Node::Unary { node: inner, .. } => references_any_const(inner, constants),
        Node::Array(items) => items.iter().any(|i| references_any_const(i, constants)),
        Node::Range(start, end) => {
            references_any_const(start, constants) || references_any_const(end, constants)
        }
        Node::Postfix { node: inner, operator } => {
            references_any_const(inner, constants)
                || references_postfix_any_const(operator, constants)
        }
        Node::Func {
            args,
            predicate,
            map_node,
            ..
        } => {
            args.iter().any(|a| references_any_const(a, constants))
                || predicate
                    .as_ref()
                    .is_some_and(|p| references_any_const_program(p, constants))
                || map_node
                    .as_ref()
                    .is_some_and(|mn| references_any_const(mn, constants))
        }
        Node::Value(_) => false,
    }
}

fn references_any_const_program(
    program: &Program,
    constants: &IndexMap<String, Node>,
) -> bool {
    program
        .lines
        .iter()
        .any(|(_, v)| references_any_const(v, constants))
        || references_any_const(&program.expr, constants)
}

fn references_postfix_any_const(
    op: &PostfixOperator,
    constants: &IndexMap<String, Node>,
) -> bool {
    match op {
        PostfixOperator::Index { idx, .. } => references_any_const(idx, constants),
        PostfixOperator::Default(n) | PostfixOperator::Pipe(n) => {
            references_any_const(n, constants)
        }
        PostfixOperator::Ternary { left, right } => {
            references_any_const(left, constants) || references_any_const(right, constants)
        }
        PostfixOperator::Range(..) => false,
    }
}

fn resolve_ident_chain(node: &Node, constants: &IndexMap<String, Node>) -> Node {
    match node {
        Node::Ident(id) => {
            if let Some(resolved) = constants.get(id.as_str()) {
                resolved.clone()
            } else {
                node.clone()
            }
        }
        Node::Operation {
            operator,
            left,
            right,
        } => {
            let l = resolve_ident_chain(left, constants);
            let r = resolve_ident_chain(right, constants);
            Node::Operation {
                operator: operator.clone(),
                left: Box::new(l),
                right: Box::new(r),
            }
        }
        Node::Unary { operator, node: inner } => Node::Unary {
            operator: operator.clone(),
            node: Box::new(resolve_ident_chain(inner, constants)),
        },
        Node::Array(items) => Node::Array(
            items
                .iter()
                .map(|i| resolve_ident_chain(i, constants))
                .collect(),
        ),
        Node::Range(start, end) => Node::Range(
            Box::new(resolve_ident_chain(start, constants)),
            Box::new(resolve_ident_chain(end, constants)),
        ),
        Node::Postfix { operator, node: inner } => Node::Postfix {
            operator: resolve_postfix_operator(operator, constants),
            node: Box::new(resolve_ident_chain(inner, constants)),
        },
        Node::Func {
            ident,
            args,
            predicate,
            threshold,
            throws,
            map_node,
        } => Node::Func {
            ident: ident.clone(),
            args: args
                .iter()
                .map(|a| resolve_ident_chain(a, constants))
                .collect(),
            predicate: predicate
                .as_ref()
                .map(|p| Box::new(resolve_ident_chain_program(p, constants))),
            threshold: *threshold,
            throws: *throws,
            map_node: map_node
                .as_ref()
                .map(|mn| Box::new(resolve_ident_chain(mn, constants))),
        },
        other => other.clone(),
    }
}

fn resolve_postfix_operator(
    op: &PostfixOperator,
    constants: &IndexMap<String, Node>,
) -> PostfixOperator {
    match op {
        PostfixOperator::Index { idx, optional } => PostfixOperator::Index {
            idx: Box::new(resolve_ident_chain(idx, constants)),
            optional: *optional,
        },
        PostfixOperator::Default(n) => {
            PostfixOperator::Default(Box::new(resolve_ident_chain(n, constants)))
        }
        PostfixOperator::Pipe(n) => {
            PostfixOperator::Pipe(Box::new(resolve_ident_chain(n, constants)))
        }
        PostfixOperator::Ternary { left, right } => PostfixOperator::Ternary {
            left: Box::new(resolve_ident_chain(left, constants)),
            right: Box::new(resolve_ident_chain(right, constants)),
        },
        PostfixOperator::Range(..) => op.clone(),
    }
}

fn resolve_ident_chain_program(
    program: &Program,
    constants: &IndexMap<String, Node>,
) -> Program {
    Program {
        lines: program
            .lines
            .iter()
            .map(|(id, val)| (id.clone(), resolve_ident_chain(val, constants)))
            .collect(),
        expr: resolve_ident_chain(&program.expr, constants),
    }
}

fn substitute_constants(node: &mut Node, constants: &IndexMap<String, Node>) -> bool {
    match node {
        Node::Ident(id) => {
            if let Some(value) = constants.get(id.as_str()) {
                *node = value.clone();
                true
            } else {
                false
            }
        }
        Node::Array(items) => items
            .iter_mut()
            .fold(false, |acc, i| substitute_constants(i, constants) || acc),
        Node::Range(start, end) => {
            let a = substitute_constants(start, constants);
            let b = substitute_constants(end, constants);
            a || b
        }
        Node::Func {
            args,
            predicate,
            map_node,
            ..
        } => {
            let mut changed = args
                .iter_mut()
                .fold(false, |acc, a| substitute_constants(a, constants) || acc);
            if let Some(p) = predicate {
                changed |= substitute_constants_program(p, constants);
            }
            if let Some(mn) = map_node {
                changed |= substitute_constants(mn, constants);
            }
            changed
        }
        Node::Unary { node: inner, .. } => substitute_constants(inner, constants),
        Node::Operation { left, right, .. } => {
            let a = substitute_constants(left, constants);
            let b = substitute_constants(right, constants);
            a || b
        }
        Node::Postfix { node: inner, operator } => {
            let a = substitute_constants(inner, constants);
            let b = substitute_postfix_constants(operator, constants);
            a || b
        }
        Node::Value(_) => false,
    }
}

fn substitute_constants_program(
    program: &mut Program,
    constants: &IndexMap<String, Node>,
) -> bool {
    let mut changed = false;
    for (_, val) in &mut program.lines {
        changed |= substitute_constants(val, constants);
    }
    changed |= substitute_constants(&mut program.expr, constants);
    changed
}

fn substitute_postfix_constants(
    op: &mut PostfixOperator,
    constants: &IndexMap<String, Node>,
) -> bool {
    match op {
        PostfixOperator::Index { idx, .. } => substitute_constants(idx, constants),
        PostfixOperator::Default(n) | PostfixOperator::Pipe(n) => {
            substitute_constants(n, constants)
        }
        PostfixOperator::Ternary { left, right } => {
            let a = substitute_constants(left, constants);
            let b = substitute_constants(right, constants);
            a || b
        }
        PostfixOperator::Range(..) => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, eval, Result};

    #[test]
    fn constant_propagation() -> Result<()> {
        assert_eq!(eval("let x = 5; x + 3", &Context::default())?.to_string(), "8");
        assert_eq!(eval("let y = 10; y * 2", &Context::default())?.to_string(), "20");
        assert_eq!(eval("let a = 1; let b = a; b + 4", &Context::default())?.to_string(), "5");
        Ok(())
    }
}
