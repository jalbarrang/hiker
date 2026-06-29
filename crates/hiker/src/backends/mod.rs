//! Backends — stage 4, generalized.
//!
//! The front end (lexer → parser → checker) is language-agnostic: it produces a
//! checked `Spec`. A **backend** turns that one `Spec` into a property-test file
//! for a specific target language and framework. Adding a language = adding a
//! backend; the front end never changes.
//!
//! Every backend emits, per law, a test that:
//!   1. generates random values per field,
//!   2. builds the argument values,
//!   3. computes the law as an ORACLE (AND of its predicates),
//!   4. calls the implementation of the same name,
//!   5. asserts implementation == oracle.

use crate::ast::Spec;

pub mod python;
pub mod rust;
pub mod typescript;

/// Knobs a backend needs that aren't in the spec itself.
pub struct EmitOptions {
    /// The system-under-test module the generated tests import from:
    /// a Rust crate name, a TS import path, or a Python module name.
    pub module: String,
}

/// A code generator for one target language.
pub trait Backend {
    /// Emit a complete, ready-to-run test file as a string.
    fn emit(&self, spec: &Spec, opts: &EmitOptions) -> String;

    /// The conventional file name for this target's generated tests,
    /// e.g. `generated.rs`. Used to build default cache paths.
    fn default_filename(&self) -> &'static str;
}

/// Resolve a `--target` name to a backend. Returns `None` for unknown targets.
pub fn for_target(target: &str) -> Option<Box<dyn Backend>> {
    match target {
        "rust" => Some(Box::new(rust::RustBackend)),
        "ts" => Some(Box::new(typescript::TypeScriptBackend)),
        "python" => Some(Box::new(python::PythonBackend)),
        _ => None,
    }
}

/// The list of known target names (for help text and validation).
pub const TARGETS: &[&str] = &["rust", "ts", "python"];
