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
    /// Shared, never mutated in place.
    ///
    /// OpenSCAD values are immutable, so a list can be shared rather than
    /// copied -- and it has to be, because the evaluator hands values back
    /// BY VALUE from every variable read and every argument bind. With a
    /// plain `Vec` each of those deep-copied the whole list, which made
    /// reading a list of n elements once per element O(n^2): 8,000 floats
    /// read 8,000 times took 0.72 s where 2,000 took 0.06, and passing the
    /// same list down a recursive function -- the only way to fold one in
    /// this language -- took 6.90 s against 0.25. Both are the same copy.
    /// `Rc` makes the clone a refcount bump and the whole shape linear.
    Vector(Rc<Vec<Value>>),
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
    /// Build a list value. The one place `Rc::new` is spelled.
    pub fn vec(items: Vec<Value>) -> Value {
        Value::Vector(Rc::new(items))
    }

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

    /// Three numeric components, padding a short vector with `fill`.
    ///
    /// translate() pads with 0 and scale() pads with 1 — the reference is
    /// explicit that scale([2,3]) "leaves Z unscaled". Sharing translate's
    /// zero-padding gave scale() a diag(2,3,0) matrix that FLATTENED every
    /// 3D child onto z=0, with a zero determinant, so neither the
    /// non-finite guard nor the winding flip noticed and the collapsed mesh
    /// went out to STL as a zero-volume soup.
    pub fn as_vec3_fill(&self, fill: f64) -> Option<[f64; 3]> {
        match self {
            Value::Vector(items) if items.len() <= 3 => {
                let mut out = [fill; 3];
                for (i, item) in items.iter().enumerate() {
                    out[i] = item.as_num()?;
                }
                Some(out)
            }
            _ => None,
        }
    }

    /// Exactly three numeric components — no zero padding.
    ///
    /// `as_vec3` pads a short vector, which is right for translate() (the
    /// reference gives it a fill value of 0) and wrong for cube(), where a
    /// 2-vector is a conversion failure rather than [x, y, 0]. Padded
    /// silently, cube([3,4]) became a zero-thickness box: empty geometry
    /// and not one word about why.
    pub fn as_vec3_exact(&self) -> Option<[f64; 3]> {
        match self {
            Value::Vector(items) if items.len() == 3 => {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A list is SHARED when a value is copied, never duplicated.
    ///
    /// This is a property test rather than a timing one, because it is the
    /// property that matters and a stopwatch would be flaky. But the timing
    /// is what found it: the evaluator returns values by value from every
    /// variable read and every argument bind, so a deep-copying list made
    /// both of those O(n). Reading 8,000 floats once each took 0.72 s and
    /// folding the same list through a recursive function took 6.90 s; with
    /// the clone a refcount bump they are 0.01 s and 0.42 s, and the gear
    /// library -- which is nothing but table lookups -- went from 1.74 s to
    /// 0.25 s for the same 10,268 triangles.
    ///
    /// Sharing is safe because the language has no mutation: a list value,
    /// once built, is never written to.
    #[test]
    fn cloning_a_list_shares_it_rather_than_copying_it() {
        let big = Value::vec((0..1000).map(|i| Value::Num(i as f64)).collect());
        let Value::Vector(first) = &big else { panic!("built a list") };
        assert_eq!(Rc::strong_count(first), 1);

        let copy = big.clone();
        let Value::Vector(second) = &copy else { panic!("cloned a list") };
        assert_eq!(Rc::strong_count(second), 2, "the clone shares the list");
        assert!(Rc::ptr_eq(first, second), "and shares the SAME allocation");

        // Nested lists share all the way down, which is what makes a table
        // of rows cheap to pass around.
        let table = Value::vec(vec![big.clone(), copy.clone()]);
        let Value::Vector(rows) = &table else { panic!() };
        let Value::Vector(row0) = &rows[0] else { panic!() };
        assert!(Rc::ptr_eq(first, row0));

        drop(copy);
        drop(table);
        assert_eq!(Rc::strong_count(first), 1, "and the count comes back down");
    }
}
