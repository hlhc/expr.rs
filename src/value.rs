use crate::Rule;
use indexmap::Equivalent;
use indexmap::IndexMap;
use log::trace;
use pest::iterators::{Pair, Pairs};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// A key in a `Value::Map`. Supports scalar types only (nil, bool, number, float, string).
/// Arrays and maps are not valid map keys.
#[derive(Debug, Clone)]
pub enum MapKey {
    Nil,
    Bool(bool),
    Number(i64),
    Float(FloatKey),
    String(String),
}

/// Wrapper around f64 that implements Eq, Hash, and Ord via bit-level comparison.
/// Two NaN values with the same bit pattern are considered equal.
#[derive(Debug, Clone, Copy)]
pub struct FloatKey(pub f64);

impl PartialEq for FloatKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FloatKey {}

impl Hash for FloatKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for FloatKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FloatKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Display for FloatKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for MapKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MapKey::Nil, MapKey::Nil) => true,
            (MapKey::Bool(a), MapKey::Bool(b)) => a == b,
            (MapKey::Number(a), MapKey::Number(b)) => a == b,
            (MapKey::Float(a), MapKey::Float(b)) => a == b,
            (MapKey::String(a), MapKey::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for MapKey {}

impl Hash for MapKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            MapKey::Nil => {}
            MapKey::Bool(b) => b.hash(state),
            MapKey::Number(n) => n.hash(state),
            MapKey::Float(f) => f.hash(state),
            MapKey::String(s) => s.hash(state),
        }
    }
}

impl PartialOrd for MapKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MapKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (MapKey::Nil, MapKey::Nil) => Ordering::Equal,
            (MapKey::Nil, _) => Ordering::Less,
            (_, MapKey::Nil) => Ordering::Greater,
            (MapKey::Bool(a), MapKey::Bool(b)) => a.cmp(b),
            (MapKey::Bool(_), _) => Ordering::Less,
            (_, MapKey::Bool(_)) => Ordering::Greater,
            (MapKey::Number(a), MapKey::Number(b)) => a.cmp(b),
            (MapKey::Number(_), _) => Ordering::Less,
            (_, MapKey::Number(_)) => Ordering::Greater,
            (MapKey::Float(a), MapKey::Float(b)) => a.cmp(b),
            (MapKey::Float(_), _) => Ordering::Less,
            (_, MapKey::Float(_)) => Ordering::Greater,
            (MapKey::String(a), MapKey::String(b)) => a.cmp(b),
        }
    }
}

impl MapKey {
    /// Convert a MapKey back to a Value (for `keys()` etc.)
    pub fn into_value(self) -> Value {
        match self {
            MapKey::Nil => Value::Nil,
            MapKey::Bool(b) => Value::Bool(b),
            MapKey::Number(n) => Value::Number(n),
            MapKey::Float(f) => Value::Float(f.0),
            MapKey::String(s) => Value::String(s),
        }
    }
}

impl Equivalent<MapKey> for str {
    fn equivalent(&self, key: &MapKey) -> bool {
        match key {
            MapKey::String(s) => self == s.as_str(),
            _ => false,
        }
    }
}

impl Equivalent<MapKey> for String {
    fn equivalent(&self, key: &MapKey) -> bool {
        match key {
            MapKey::String(s) => self == s,
            _ => false,
        }
    }
}

impl Equivalent<MapKey> for i64 {
    fn equivalent(&self, key: &MapKey) -> bool {
        match key {
            MapKey::Number(n) => *self == *n,
            _ => false,
        }
    }
}

impl Equivalent<MapKey> for bool {
    fn equivalent(&self, key: &MapKey) -> bool {
        match key {
            MapKey::Bool(b) => *self == *b,
            _ => false,
        }
    }
}

impl Equivalent<MapKey> for f64 {
    fn equivalent(&self, key: &MapKey) -> bool {
        match key {
            MapKey::Float(f) => self.to_bits() == f.0.to_bits(),
            _ => false,
        }
    }
}

impl Display for MapKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MapKey::Nil => write!(f, "nil"),
            MapKey::Bool(b) => write!(f, "{b}"),
            MapKey::Number(n) => write!(f, "{n}"),
            MapKey::Float(n) => write!(f, "{}", n.0),
            MapKey::String(s) => write!(f, "{s}"),
        }
    }
}

impl From<String> for MapKey {
    fn from(s: String) -> Self {
        MapKey::String(s)
    }
}

impl From<&str> for MapKey {
    fn from(s: &str) -> Self {
        MapKey::String(s.to_string())
    }
}

impl From<i64> for MapKey {
    fn from(n: i64) -> Self {
        MapKey::Number(n)
    }
}

impl From<i32> for MapKey {
    fn from(n: i32) -> Self {
        MapKey::Number(n as i64)
    }
}

impl From<bool> for MapKey {
    fn from(b: bool) -> Self {
        MapKey::Bool(b)
    }
}

impl From<f64> for MapKey {
    fn from(f: f64) -> Self {
        MapKey::Float(FloatKey(f))
    }
}

impl TryFrom<Value> for MapKey {
    type Error = String;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Nil => Ok(MapKey::Nil),
            Value::Bool(b) => Ok(MapKey::Bool(b)),
            Value::Number(n) => Ok(MapKey::Number(n)),
            Value::Float(f) => Ok(MapKey::Float(FloatKey(f))),
            Value::String(s) => Ok(MapKey::String(s)),
            Value::Array(_) => Err("cannot use array as map key".to_string()),
            Value::Map(_) => Err("cannot use map as map key".to_string()),
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for MapKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MapKey::Nil => serializer.serialize_str("nil"),
            MapKey::Bool(b) => serializer.serialize_str(if *b { "true" } else { "false" }),
            MapKey::Number(n) => serializer.collect_str(n),
            MapKey::Float(f) => serializer.collect_str(&f.0),
            MapKey::String(s) => serializer.serialize_str(s),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MapKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(MapKey::String(s))
    }
}

/// Represents a data value as input or output to an expr program
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Value {
    Number(i64),
    Bool(bool),
    Float(f64),
    #[default]
    Nil,
    String(String),
    Array(Vec<Value>),
    Map(IndexMap<MapKey, Value>),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&IndexMap<MapKey, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
}

impl<K, V> FromIterator<(K, V)> for Value
where
    K: Into<String>,
    V: Into<Value>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        Value::Map(
            iter.into_iter()
                .map(|(k, v)| (MapKey::String(k.into()), v.into()))
                .collect(),
        )
    }
}

impl AsRef<Value> for Value {
    fn as_ref(&self) -> &Value {
        self
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n as i64)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Number(n as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&String> for Value {
    fn from(s: &String) -> Self {
        s.to_string().into()
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl<V: Into<Value>> From<Vec<V>> for Value {
    fn from(a: Vec<V>) -> Self {
        Value::Array(a.into_iter().map(|v| v.into()).collect())
    }
}

impl From<IndexMap<MapKey, Value>> for Value {
    fn from(m: IndexMap<MapKey, Value>) -> Self {
        Value::Map(m)
    }
}

impl From<IndexMap<String, Value>> for Value {
    fn from(m: IndexMap<String, Value>) -> Self {
        Value::Map(m.into_iter().map(|(k, v)| (MapKey::String(k), v)).collect())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => write!(f, "nil"),
            Value::String(s) => write!(
                f,
                r#""{}""#,
                s.replace("\\", "\\\\")
                    .replace("\n", "\\n")
                    .replace("\r", "\\r")
                    .replace("\t", "\\t")
                    .replace("\"", "\\\"")
            ),
            Value::Array(a) => write!(
                f,
                "[{}]",
                a.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Map(m) => write!(
                f,
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl From<Pairs<'_, Rule>> for Value {
    fn from(mut pairs: Pairs<Rule>) -> Self {
        pairs.next().unwrap().into()
    }
}

impl From<Pair<'_, Rule>> for Value {
    fn from(pair: Pair<Rule>) -> Self {
        trace!("{:?} = {}", &pair.as_rule(), pair.as_str());
        match pair.as_rule() {
            Rule::literal => pair.into_inner().into(),
            Rule::nil => Value::Nil,
            Rule::bool => Value::Bool(pair.as_str().parse().unwrap()),
            Rule::int => Value::Number(pair.as_str().replace('_', "").parse().unwrap()),
            Rule::decimal => Value::Float(pair.as_str().replace('_', "").parse().unwrap()),
            Rule::string_multiline => pair.into_inner().as_str().into(),
            Rule::string => pair
                .into_inner()
                .as_str()
                .replace("\\\\", "\\")
                .replace("\\n", "\n")
                .replace("\\r", "\r")
                .replace("\\t", "\t")
                .replace("\\\"", "\"")
                .into(),
            rule => unreachable!("Unexpected rule: {rule:?} {}", pair.as_str()),
        }
    }
}
