use serde_json;

use crate::functions::FunctionMetadata;
use crate::{Environment, MapKey, Value, bail};

fn const_len(args: &[Value]) -> Option<Value> {
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        Value::Array(a) => Some(Value::Number(a.len() as i64)),
        Value::String(s) => Some(Value::Number(s.len() as i64)),
        Value::Map(m) => Some(Value::Number(m.len() as i64)),
        _ => None,
    }
}

fn const_keys(args: &[Value]) -> Option<Value> {
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        Value::Map(m) => Some(Value::Array(
            m.keys().map(|k| k.clone().into_value()).collect(),
        )),
        _ => None,
    }
}

fn const_values(args: &[Value]) -> Option<Value> {
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        Value::Map(m) => Some(Value::Array(m.values().cloned().collect())),
        _ => None,
    }
}

/// Convert a serde_json::Value to an expr::Value
fn json_to_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Nil
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => Value::Map(
            obj.into_iter()
                .map(|(k, v)| (MapKey::String(k), json_to_value(v)))
                .collect(),
        ),
    }
}

/// Convert an expr::Value to a serde_json::Value
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => serde_json::Value::Number((*n).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
        Value::Map(m) => serde_json::Value::Object(
            m.iter()
                .map(|(k, v)| (k.to_string(), value_to_json(v)))
                .collect(),
        ),
    }
}

pub fn add_json_functions(env: &mut Environment) {
    // fromJSON(string) -> Value
    // Parses a JSON string and returns the corresponding Value
    env.add_function("fromJSON", |c| {
        if c.args.len() != 1 {
            bail!("fromJSON() takes exactly one argument");
        }
        if let Value::String(s) = &c.args[0] {
            match serde_json::from_str::<serde_json::Value>(s) {
                Ok(json) => Ok(json_to_value(json)),
                Err(e) => bail!("fromJSON() failed to parse JSON: {}", e),
            }
        } else {
            bail!("fromJSON() takes a string as the argument");
        }
    });

    // toJSON(value) -> String
    // Serializes a value to a JSON string
    env.add_function("toJSON", |c| {
        if c.args.len() != 1 {
            bail!("toJSON() takes exactly one argument");
        }
        let json = value_to_json(&c.args[0]);
        match serde_json::to_string(&json) {
            Ok(s) => Ok(Value::String(s)),
            Err(e) => bail!("toJSON() failed to serialize: {}", e),
        }
    });

    // keys(map) -> Array of keys
    // Returns an array of the keys in a map
    env.add_builtin_function(
        "keys",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_keys(args) {
                return Ok(result);
            }
            bail!("keys() takes a map as the argument")
        },
        FunctionMetadata {
            const_eval: Some(const_keys),
        },
    );

    // values(map) -> Array of values
    // Returns an array of the values in a map
    env.add_builtin_function(
        "values",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_values(args) {
                return Ok(result);
            }
            bail!("values() takes a map as the argument")
        },
        FunctionMetadata {
            const_eval: Some(const_values),
        },
    );

    // len(array|string|map) -> Number
    // Returns the length of an array, string, or map
    env.add_builtin_function(
        "len",
        |c| {
            let args = c.args.as_slice();
            if let Some(result) = const_len(args) {
                return Ok(result);
            }
            bail!("len() takes an array, string, or map as the argument")
        },
        FunctionMetadata {
            const_eval: Some(const_len),
        },
    );
}
