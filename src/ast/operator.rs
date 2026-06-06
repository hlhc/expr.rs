use crate::Value::{Array, Bool, Float, Map, Number, String};
use crate::ast::node::Node;
use crate::{Context, Environment, Value};
use crate::{MapKey, Result, Rule, bail};
use log::trace;
use pest::iterators::Pair;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, strum::EnumString, strum::Display)]
pub enum Operator {
    #[strum(serialize = "+")]
    Add,
    #[strum(serialize = "-")]
    Subtract,
    #[strum(serialize = "*")]
    Multiply,
    #[strum(serialize = "/")]
    Divide,
    #[strum(serialize = "%")]
    Modulo,
    #[strum(serialize = "^")]
    Pow,
    #[strum(serialize = "==")]
    Equal,
    #[strum(serialize = "!=")]
    NotEqual,
    #[strum(serialize = ">")]
    GreaterThan,
    #[strum(serialize = ">=")]
    GreaterThanOrEqual,
    #[strum(serialize = "<")]
    LessThan,
    #[strum(serialize = "<=")]
    LessThanOrEqual,
    #[strum(serialize = "&&", serialize = "and")]
    And,
    #[strum(serialize = "||", serialize = "or")]
    Or,
    #[strum(serialize = "in")]
    In,
    #[strum(serialize = "contains")]
    Contains,
    #[strum(serialize = "startsWith")]
    StartsWith,
    #[strum(serialize = "endsWith")]
    EndsWith,
    #[strum(serialize = "matches")]
    Matches,
}

impl From<Pair<'_, Rule>> for Operator {
    fn from(pair: Pair<Rule>) -> Self {
        trace!("[operator] {pair:?}");
        match pair.as_str() {
            "**" => Operator::Pow,
            op => Operator::from_str(op).unwrap_or_else(|_| unreachable!("Invalid operator {op}")),
        }
    }
}

impl Environment<'_> {
    pub fn eval_operator(
        &self,
        ctx: &Context,
        operator: Operator,
        left: Node,
        right: Node,
    ) -> Result<Value> {
        // Short-circuit evaluation for logical operators
        match operator {
            Operator::And => {
                let left = self.eval_expr(ctx, left)?;
                if left.as_bool() != Some(true) {
                    return Ok(Value::Bool(false));
                }
                let right = self.eval_expr(ctx, right)?;
                Ok(Value::Bool(right.as_bool() == Some(true)))
            }
            Operator::Or => {
                let left = self.eval_expr(ctx, left)?;
                if left.as_bool() == Some(true) {
                    return Ok(Value::Bool(true));
                }
                let right = self.eval_expr(ctx, right)?;
                Ok(Value::Bool(right.as_bool() == Some(true)))
            }
            _ => {
                let left = self.eval_expr(ctx, left)?;
                let right = self.eval_expr(ctx, right)?;
                let result = match operator {
                    Operator::Add => match (left, right) {
                        (Number(left), Number(right)) => (left + right).into(),
                        (Float(left), Float(right)) => (left + right).into(),
                        (String(left), String(right)) => format!("{left}{right}").into(),
                        _ => bail!("Invalid operands for operator +"),
                    },
                    Operator::Subtract => match (left, right) {
                        (Number(left), Number(right)) => Number(left - right),
                        (Float(left), Float(right)) => Float(left - right),
                        _ => bail!("Invalid operands for operator -"),
                    },
                    Operator::Multiply => match (left, right) {
                        (Number(left), Number(right)) => Number(left * right),
                        (Float(left), Float(right)) => Float(left * right),
                        _ => bail!("Invalid operands for operator *"),
                    },
                    Operator::Divide => match (left, right) {
                        (Number(left), Number(right)) => Number(left / right),
                        (Float(left), Float(right)) => Float(left / right),
                        _ => bail!("Invalid operands for operator /"),
                    },
                    Operator::Modulo => match (left, right) {
                        (Number(left), Number(right)) => Number(left % right),
                        _ => bail!("Invalid operands for operator %"),
                    },
                    Operator::Pow => match (left, right) {
                        (Number(left), Number(right)) => Number(left.pow(right as u32)),
                        (Float(left), Float(right)) => Float(left.powf(right)),
                        _ => bail!("Invalid operands for operator {operator}"),
                    },
                    Operator::Equal => Bool(left == right),
                    Operator::NotEqual => Bool(left != right),
                    Operator::GreaterThan => match (left, right) {
                        (Number(left), Number(right)) => (left > right).into(),
                        (Float(left), Float(right)) => (left > right).into(),
                        (String(left), String(right)) => (left > right).into(),
                        _ => bail!("Invalid operands for operator {operator}"),
                    },
                    Operator::GreaterThanOrEqual => match (left, right) {
                        (Number(left), Number(right)) => (left >= right).into(),
                        (Float(left), Float(right)) => (left >= right).into(),
                        (String(left), String(right)) => (left >= right).into(),
                        _ => bail!("Invalid operands for operator {operator}"),
                    },
                    Operator::LessThan => match (left, right) {
                        (Number(left), Number(right)) => (left < right).into(),
                        (Float(left), Float(right)) => (left < right).into(),
                        (String(left), String(right)) => (left < right).into(),
                        _ => bail!("Invalid operands for operator {operator}"),
                    },
                    Operator::LessThanOrEqual => match (left, right) {
                        (Number(left), Number(right)) => (left <= right).into(),
                        (Float(left), Float(right)) => (left <= right).into(),
                        (String(left), String(right)) => (left <= right).into(),
                        _ => bail!("Invalid operands for operator {operator}"),
                    },
                    Operator::In => match (left, right) {
                        (String(left), Map(right)) => {
                            right.contains_key(&MapKey::String(left.clone())).into()
                        }
                        (Number(left), Map(right)) => {
                            right.contains_key(&MapKey::Number(left)).into()
                        }
                        (left, Array(right)) => right.contains(&left).into(),
                        _ => bail!("Invalid operands for operator {operator}"),
                    },
                    Operator::Contains => match (left, right) {
                        (String(left), String(right)) => left.contains(&right).into(),
                        (Array(left), right) => left.contains(&right).into(),
                        (Map(left), String(right)) => {
                            left.contains_key(&MapKey::String(right)).into()
                        }
                        _ => bail!("Invalid operands for operator contains"),
                    },
                    Operator::StartsWith => match (left, right) {
                        (String(left), String(right)) => Bool(left.starts_with(&right)),
                        _ => bail!("Invalid operands for operator startsWith"),
                    },
                    Operator::EndsWith => match (left, right) {
                        (String(left), String(right)) => Bool(left.ends_with(&right)),
                        _ => bail!("Invalid operands for operator endsWith"),
                    },
                    Operator::Matches => match (left, right) {
                        (String(left), String(right)) => {
                            let re = regex::Regex::new(&right)?;
                            Bool(re.is_match(&left))
                        }
                        _ => bail!("Invalid operands for operator matches"),
                    },
                    Operator::And | Operator::Or => unreachable!(),
                };

                Ok(result)
            }
        }
    }
}
