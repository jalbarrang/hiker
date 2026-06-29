//! The AST — Abstract Syntax Tree.
//!
//! This is the structured, in-memory shape of a parsed `.tent` file. The lexer
//! gave us a flat list of tokens; the parser (next stage) turns that list into
//! this tree. Every later stage (checker, codegen) reads *this*, never the raw
//! text or tokens.
//!
//! A whole spec is three lists: the sorts, the relations, and the laws.

/// A complete parsed spec.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub sorts: Vec<Sort>,
    pub relations: Vec<Relation>,
    pub laws: Vec<Law>,
}

/// `sort MediaItem` or `sort TemporalPoint { media: MediaItem, t: Int }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sort {
    pub name: String,
    pub fields: Vec<Field>,
    pub line: usize,
}

/// A single field inside a sort, e.g. `t: Int`.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
}

/// A field's type: either the built-in `Int` or a reference to another sort.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Sort(String),
}

/// `relation point_in_interval(p: TemporalPoint, i: TemporalInterval)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    pub name: String,
    pub params: Vec<Param>,
    pub line: usize,
}

/// One parameter of a relation, e.g. `p: TemporalPoint`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub sort: String,
}

/// `law point_in_interval(p, i) { ...predicates... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Law {
    pub relation: String,
    pub args: Vec<String>,
    pub preds: Vec<Pred>,
    pub line: usize,
}

/// One predicate line inside a law body, e.g. `i.t0 <= p.t`.
/// All predicates in a law are implicitly AND-ed together.
#[derive(Debug, Clone, PartialEq)]
pub struct Pred {
    pub lhs: Expr,
    pub op: CmpOp,
    pub rhs: Expr,
    pub line: usize,
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq, // ==
    Le, // <=
    Lt, // <
    Ge, // >=
    Gt, // >
}

/// An expression appearing on either side of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `p.media` — access a field of a law argument.
    Field { arg: String, field: String },
    /// An integer literal.
    Int(i64),
    /// A bare argument name, e.g. comparing `a == b` whole-entity.
    Arg(String),
}
