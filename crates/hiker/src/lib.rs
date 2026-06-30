//! hiker — a tiny DSL for stating architectural intent.
//!
//! Pipeline (one module per compiler stage; build them in this order):
//!   lexer   : &str        -> Vec<Token>        (turn text into words)
//!   parser  : Vec<Token>  -> Spec (the AST)    (turn words into structure)
//!   checker : &Spec       -> Result<(), _>     ("intent compiles")
//!   backends: &Spec       -> String (tests)     (the bridge to real code)
//!
//! The front end (lexer/parser/checker) is language-agnostic; only the chosen
//! backend is target-specific.

// Tests use `.unwrap()` freely for brevity; production code may not (see the
// disallowed-methods ban in clippy.toml).
#![cfg_attr(test, allow(clippy::disallowed_methods))]

pub mod ast;
pub mod backends;
pub mod checker;
pub mod eval;
pub mod facts;
pub mod lexer;
pub mod parser;
pub mod verify;
