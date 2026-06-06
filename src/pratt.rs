use crate::Rule;
use once_cell::sync::Lazy;
use pest::pratt_parser::PrattParser;

pub(crate) static PRATT_PARSER: Lazy<PrattParser<Rule>> = Lazy::new(|| {
    use Rule::*;
    use pest::pratt_parser::{Assoc::*, Op};

    PrattParser::new()
        .op(Op::postfix(ternary_op))
        .op(Op::postfix(pipe_op))
        .op(Op::infix(logical_or_op, Left))
        .op(Op::infix(logical_and_op, Left))
        .op(Op::infix(equality_op, Left)
            | Op::infix(comparison_op, Left)
            | Op::infix(relation_op, Left))
        .op(Op::infix(range_op, Left))
        .op(Op::infix(addition_op, Left))
        .op(Op::prefix(not_op))
        .op(Op::infix(multiplication_op, Left))
        .op(Op::prefix(sign_op))
        .op(Op::infix(exponent_op, Right))
        .op(Op::postfix(default_op))
        .op(Op::postfix(member_op)
            | Op::postfix(index_op)
            | Op::postfix(optional_member_op)
            | Op::postfix(optional_index_op)
            | Op::postfix(slice_from_op)
            | Op::postfix(slice_to_op))
});
