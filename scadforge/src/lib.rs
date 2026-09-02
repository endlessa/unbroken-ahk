//! scadforge — a from-scratch, zero-dependency, OpenSCAD-compatible
//! modeler served as a local web app.
//!
//! Pipeline: lexer → parser → evaluator → geometry kernel → mesh JSON →
//! browser viewport. Semantics follow the project's converged OpenSCAD
//! 2021.01 language reference (docs/openscad_language_reference.json);
//! this crate is the first vertical slice of the blueprint in
//! docs/OPENSCAD_BLUEPRINT.html.

pub mod ast;
pub mod eval;
pub mod geom;
pub mod http;
pub mod lexer;
pub mod parser;
pub mod value;
