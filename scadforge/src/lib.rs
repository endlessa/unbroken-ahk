//! scadforge — a from-scratch, zero-dependency, OpenSCAD-compatible
//! modeler served as a local web app.
//!
//! Pipeline: lexer → parser → evaluator → geometry kernel → mesh JSON →
//! browser viewport. Semantics follow the project's converged OpenSCAD
//! 2021.01 language reference (docs/openscad_language_reference.json);
//! this crate is the first vertical slice of the blueprint in
//! docs/OPENSCAD_BLUEPRINT.html.

pub mod ast;
pub mod csg;
pub mod csg2;
pub mod csgfmt;
pub mod customizer;
pub mod deflate;
pub mod eval;
pub mod font;
pub mod geom;
pub mod http;
pub mod io;
pub mod lexer;
pub mod parser;
pub mod preproc;
pub mod offset;
pub mod poly2;
pub mod svg;
pub mod value;
pub mod zip;
