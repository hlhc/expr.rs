use crate::functions::FunctionMetadata;
use crate::{Environment, Value, bail};

fn const_trim(args: &[Value]) -> Option<Value> {
    if args.is_empty() || args.len() > 2 {
        return None;
    }
    let Value::String(s) = &args[0] else {
        return None;
    };
    if args.len() == 1 {
        Some(Value::String(s.trim().to_string()))
    } else if let Value::String(chars) = &args[1] {
        Some(Value::String(
            s.trim_matches(|c| chars.contains(c)).to_string(),
        ))
    } else {
        None
    }
}

fn const_trim_prefix(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => Some(Value::String(
            s.strip_prefix(prefix.as_str()).unwrap_or(s).to_string(),
        )),
        _ => None,
    }
}

fn const_trim_suffix(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => Some(Value::String(
            s.strip_suffix(suffix.as_str()).unwrap_or(s).to_string(),
        )),
        _ => None,
    }
}

fn const_upper(args: &[Value]) -> Option<Value> {
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        Value::String(s) => Some(Value::String(s.to_uppercase())),
        _ => None,
    }
}

fn const_lower(args: &[Value]) -> Option<Value> {
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        Value::String(s) => Some(Value::String(s.to_lowercase())),
        _ => None,
    }
}

fn const_replace(args: &[Value]) -> Option<Value> {
    if args.len() != 3 {
        return None;
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::String(from), Value::String(to)) => {
            Some(Value::String(s.replace(from.as_str(), to.as_str())))
        }
        _ => None,
    }
}

fn const_repeat(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::Number(n)) => Some(Value::String(s.repeat(*n as usize + 1))),
        _ => None,
    }
}

fn const_index_of(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(sub)) => Some(Value::Number(
            s.find(sub.as_str()).map(|i| i as i64).unwrap_or(-1),
        )),
        _ => None,
    }
}

fn const_last_index_of(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(sub)) => Some(Value::Number(
            s.rfind(sub.as_str()).map(|i| i as i64).unwrap_or(-1),
        )),
        _ => None,
    }
}

fn const_has_prefix(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => {
            Some(Value::Bool(s.starts_with(prefix.as_str())))
        }
        _ => None,
    }
}

fn const_has_suffix(args: &[Value]) -> Option<Value> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => {
            Some(Value::Bool(s.ends_with(suffix.as_str())))
        }
        _ => None,
    }
}

pub fn add_string_functions(env: &mut Environment) {
    env.add_builtin_function(
        "trim",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_trim(args) {
                return Ok(result);
            }
            bail!("trim() takes a string as the first argument and an optional string of characters to trim")
        },
        FunctionMetadata {
            const_eval: Some(const_trim),
        },
    );

    env.add_builtin_function(
        "trimPrefix",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_trim_prefix(args) {
                return Ok(result);
            }
            bail!("trimPrefix() takes a string as the first argument and a string to trim as the second argument")
        },
        FunctionMetadata {
            const_eval: Some(const_trim_prefix),
        },
    );

    env.add_builtin_function(
        "trimSuffix",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_trim_suffix(args) {
                return Ok(result);
            }
            bail!("trimSuffix() takes a string as the first argument and a string to trim as the second argument")
        },
        FunctionMetadata {
            const_eval: Some(const_trim_suffix),
        },
    );

    env.add_builtin_function(
        "upper",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_upper(args) {
                return Ok(result);
            }
            bail!("upper() takes a string as the first argument")
        },
        FunctionMetadata {
            const_eval: Some(const_upper),
        },
    );

    env.add_builtin_function(
        "lower",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_lower(args) {
                return Ok(result);
            }
            bail!("lower() takes a string as the first argument")
        },
        FunctionMetadata {
            const_eval: Some(const_lower),
        },
    );

    env.add_builtin_function(
        "replace",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_replace(args) {
                return Ok(result);
            }
            bail!("replace() takes a string as the first argument and two strings to replace")
        },
        FunctionMetadata {
            const_eval: Some(const_replace),
        },
    );

    env.add_builtin_function(
        "repeat",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_repeat(args) {
                return Ok(result);
            }
            bail!(
                "repeat() takes a string as the first argument and a number as the second argument"
            )
        },
        FunctionMetadata {
            const_eval: Some(const_repeat),
        },
    );

    env.add_builtin_function(
        "indexOf",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_index_of(args) {
                return Ok(result);
            }
            bail!("indexOf() takes a string as the first argument and a string to search for as the second argument")
        },
        FunctionMetadata {
            const_eval: Some(const_index_of),
        },
    );

    env.add_builtin_function(
        "lastIndexOf",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_last_index_of(args) {
                return Ok(result);
            }
            bail!("lastIndexOf() takes a string as the first argument and a string to search for as the second argument")
        },
        FunctionMetadata {
            const_eval: Some(const_last_index_of),
        },
    );

    env.add_builtin_function(
        "hasPrefix",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_has_prefix(args) {
                return Ok(result);
            }
            bail!("hasPrefix() takes a string as the first argument and a string to search for as the second argument")
        },
        FunctionMetadata {
            const_eval: Some(const_has_prefix),
        },
    );

    env.add_builtin_function(
        "hasSuffix",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_has_suffix(args) {
                return Ok(result);
            }
            bail!("hasSuffix() takes a string as the first argument and a string to search for as the second argument")
        },
        FunctionMetadata {
            const_eval: Some(const_has_suffix),
        },
    );

    // Non-pure functions: split, splitAfter
    env.add_function("split", |c| {
        if let (Value::String(s), Value::String(sep), None) =
            (&c.args[0], &c.args[1], c.args.get(2))
        {
            Ok(s.split(sep).map(Value::from).collect::<Vec<_>>().into())
        } else if let (Value::String(s), Value::String(sep), Some(Value::Number(n))) =
            (&c.args[0], &c.args[1], c.args.get(2))
        {
            Ok(s.splitn(*n as usize, sep)
                .map(Value::from)
                .collect::<Vec<_>>()
                .into())
        } else {
            bail!(
                "split() takes a string as the first argument and a string as the second argument"
            );
        }
    });

    env.add_function("splitAfter", |c| {
        if let (Value::String(s), Value::String(sep), None) =
            (&c.args[0], &c.args[1], c.args.get(2))
        {
            Ok(s.split_inclusive(sep).map(Value::from).collect::<Vec<_>>().into())
        } else if let (Value::String(s), Value::String(sep), Some(Value::Number(n))) =
            (&c.args[0], &c.args[1], c.args.get(2))
        {
            let mut arr = s
                .split_inclusive(sep)
                .take(*n as usize - 1)
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            arr.push(
                s.split_inclusive(sep)
                    .skip(*n as usize - 1)
                    .collect::<Vec<_>>()
                    .join(""),
            );
            Ok(arr.into())
        } else {
            bail!("splitAfter() takes a string as the first argument and a string as the second argument");
        }
    });
}
