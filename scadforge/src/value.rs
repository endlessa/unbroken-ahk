//! Runtime values for the SCAD-compatible subset.

use crate::ast::{Expr, Param};
use std::rc::Rc;

/// A first-class function value (2021.01 function literals). Captures the
/// lexical scope at the definition site; `$`-names still resolve
/// dynamically at each call.
pub struct FuncVal {
    pub params: Vec<Param>,
    pub body: Expr,
    /// The captured lexical environment (an eval::Scope), type-erased so
    /// value.rs does not depend on the evaluator's scope type.
    pub env: Rc<dyn std::any::Any>,
}

impl std::fmt::Debug for FuncVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncVal").field("params", &self.params).finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Str(String),
    Vector(Vec<Value>),
    /// implicit_step records the two-part [a:b] spelling — only that form
    /// gets the legacy reversed-range swap; [10:1:0] iterates zero times.
    /// Semantic equality (value_eq) compares begin/step/end only.
    Range { start: f64, step: f64, end: f64, implicit_step: bool },
    /// Function value: equality is identity (two literals are never equal
    /// unless they are the same evaluation result).
    Function(Rc<FuncVal>),
    Undef,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Num(a), Value::Num(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => a == b,
            (
                Value::Range { start: a, step: b, end: c, .. },
                Value::Range { start: d, step: e, end: f, .. },
            ) => a == d && b == e && c == f,
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Undef, Value::Undef) => true,
            _ => false,
        }
    }
}

impl Value {
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// A numeric 3-vector; shorter vectors zero-fill (the reference's
    /// rotate([90]) behavior), longer ones are rejected by callers that
    /// care.
    pub fn as_vec3(&self) -> Option<[f64; 3]> {
        match self {
            Value::Vector(items) if items.len() <= 3 => {
                let mut out = [0.0; 3];
                for (i, item) in items.iter().enumerate() {
                    out[i] = item.as_num()?;
                }
                Some(out)
            }
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Str(_) => "string",
            Value::Vector(_) => "vector",
            Value::Range { .. } => "range",
            Value::Function(_) => "function",
            Value::Undef => "undef",
        }
    }
}
