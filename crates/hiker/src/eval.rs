//! Eval — a runtime interpreter for laws.
//!
//! The backends *emit* a law as target-language source (`clause_to_rust` etc.);
//! they never run it. `verify` needs the opposite: given concrete facts, compute
//! whether a law actually holds. So this module is a small interpreter over a
//! `Binding` (law argument name → a concrete `Instance`).
//!
//! The one shared obligation with the backends: an implication `a => b` MUST
//! lower to `!a || b` here exactly as it does in codegen (AGENTS.md: "backends
//! must agree on lowering" — the interpreter is bound by the same rule). A unit
//! test pins this.
//!
//! Nothing here panics on bad data: an unresolved argument, a missing field, or
//! an ordering comparison on a non-integer returns `Err(String)` so `verify` can
//! turn it into a reported violation rather than crashing.

use std::collections::HashMap;

use crate::ast::*;

/// A concrete value a law expression resolves to. An instance's identity is its
/// id string; every readable field is an `i64` (sort-typed fields are carried as
/// integer ids in the fact model, mirroring the backends' u32-id lowering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Id(String),
}

/// One concrete entity: a stable identity plus its integer field values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub id: String,
    pub fields: HashMap<String, i64>,
}

/// Maps a law's argument names to the concrete instances bound to them for one
/// evaluation. Borrowed, because a single population is evaluated many times.
pub struct Binding<'a> {
    map: HashMap<&'a str, &'a Instance>,
}

impl<'a> Binding<'a> {
    /// Bind `args` positionally to `instances`. Arity must match (the caller —
    /// `verify` — derives both from the same relation, so a mismatch is a bug).
    pub fn new(args: &'a [String], instances: &[&'a Instance]) -> Result<Self, String> {
        if args.len() != instances.len() {
            return Err(format!(
                "binding arity mismatch: {} argument(s), {} instance(s)",
                args.len(),
                instances.len()
            ));
        }
        let mut map = HashMap::new();
        for (arg, inst) in args.iter().zip(instances.iter()) {
            map.insert(arg.as_str(), *inst);
        }
        Ok(Self { map })
    }

    fn instance(&self, arg: &str) -> Result<&Instance, String> {
        self.map
            .get(arg)
            .copied()
            .ok_or_else(|| format!("unknown argument `{arg}`"))
    }
}

/// The result of evaluating a law (or any part of it): the truth value, or an
/// error describing a fact gap that made evaluation impossible.
pub type EvalResult = Result<bool, String>;

/// Evaluate a whole law under a binding: the AND of all its clauses. An empty
/// body is `true` — matching the backends' `true` oracle for a law with no
/// clauses (though the checker rejects empty bodies upstream).
pub fn eval_law(law: &Law, b: &Binding) -> EvalResult {
    for clause in &law.clauses {
        if !eval_clause(clause, b)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn eval_clause(clause: &Clause, b: &Binding) -> EvalResult {
    match clause {
        Clause::Compare(p) => eval_pred(p, b),
        // `antecedent => consequent` is `(!antecedent || consequent)` — the same
        // lowering the backends emit. Do not change one without the other.
        Clause::Implies { ante, cons, .. } => Ok(!eval_pred(ante, b)? || eval_pred(cons, b)?),
    }
}

fn eval_pred(pred: &Pred, b: &Binding) -> EvalResult {
    let lhs = resolve(&pred.lhs, b)?;
    let rhs = resolve(&pred.rhs, b)?;
    match pred.op {
        // Equality compares like with like: two ints, or two identities.
        CmpOp::Eq => match (&lhs, &rhs) {
            (Value::Int(a), Value::Int(c)) => Ok(a == c),
            (Value::Id(a), Value::Id(c)) => Ok(a == c),
            _ => Err(format!(
                "line {}: `==` compares mismatched value kinds",
                pred.line
            )),
        },
        // Ordering requires integers on both sides. Ordering two identities is a
        // type error (mirrors the checker's rule for ordering on non-Int).
        op => {
            let (Value::Int(l), Value::Int(r)) = (&lhs, &rhs) else {
                return Err(format!(
                    "line {}: comparison `{}` needs integers",
                    pred.line,
                    op_str(op)
                ));
            };
            Ok(match op {
                CmpOp::Le => l <= r,
                CmpOp::Lt => l < r,
                CmpOp::Ge => l >= r,
                CmpOp::Gt => l > r,
                CmpOp::Eq => l == r, // handled above; keeps the match total
            })
        }
    }
}

/// Resolve an expression to a concrete value under the binding.
fn resolve(expr: &Expr, b: &Binding) -> Result<Value, String> {
    match expr {
        Expr::Int(n) => Ok(Value::Int(*n)),
        Expr::Arg(name) => Ok(Value::Id(b.instance(name)?.id.clone())),
        Expr::Field { arg, field } => {
            let inst = b.instance(arg)?;
            match inst.fields.get(field) {
                Some(v) => Ok(Value::Int(*v)),
                None => Err(format!("instance `{}` has no field `{field}`", inst.id)),
            }
        }
    }
}

fn op_str(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::Le => "<=",
        CmpOp::Lt => "<",
        CmpOp::Ge => ">=",
        CmpOp::Gt => ">",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// Build an instance with the given id and integer fields.
    fn inst(id: &str, fields: &[(&str, i64)]) -> Instance {
        Instance {
            id: id.to_string(),
            fields: fields.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        }
    }

    fn first_law(src: &str) -> Law {
        parse(src).unwrap().laws.into_iter().next().unwrap()
    }

    #[test]
    fn ordering_law_holds_inward_and_fails_outward() {
        let law = first_law(
            "\
sort Module { layer: Int }
relation depends_on(importer: Module, imported: Module)
law depends_on(importer, imported) { imported.layer <= importer.layer }",
        );

        // importer.layer = 2, imported.layer = 1 → 1 <= 2 holds.
        let importer = inst("a", &[("layer", 2)]);
        let imported = inst("b", &[("layer", 1)]);
        let b = Binding::new(&law.args, &[&importer, &imported]).unwrap();
        assert_eq!(eval_law(&law, &b), Ok(true));

        // importer.layer = 0, imported.layer = 2 → 2 <= 0 is violated.
        let importer = inst("a", &[("layer", 0)]);
        let imported = inst("b", &[("layer", 2)]);
        let b = Binding::new(&law.args, &[&importer, &imported]).unwrap();
        assert_eq!(eval_law(&law, &b), Ok(false));
    }

    #[test]
    fn implication_lowers_to_not_ante_or_cons() {
        let law = first_law(
            "\
sort E { tag: Int, status: Int }
relation det(a: E, b: E)
law det(a, b) { a.tag == b.tag => a.status == b.status }",
        );
        let eval = |a: Instance, c: Instance| {
            let b = Binding::new(&law.args, &[&a, &c]).unwrap();
            eval_law(&law, &b)
        };

        // tags differ → antecedent false → !false || _ == true (vacuous).
        assert_eq!(
            eval(
                inst("a", &[("tag", 1), ("status", 5)]),
                inst("b", &[("tag", 2), ("status", 9)])
            ),
            Ok(true)
        );
        // tags equal, statuses equal → true && true.
        assert_eq!(
            eval(
                inst("a", &[("tag", 1), ("status", 5)]),
                inst("b", &[("tag", 1), ("status", 5)])
            ),
            Ok(true)
        );
        // tags equal, statuses differ → !true || false == false.
        assert_eq!(
            eval(
                inst("a", &[("tag", 1), ("status", 5)]),
                inst("b", &[("tag", 1), ("status", 9)])
            ),
            Ok(false)
        );
    }

    #[test]
    fn empty_body_is_true() {
        let law = first_law(
            "\
sort P
relation r(a: P)
law r(a) { }",
        );
        let p = inst("x", &[]);
        let b = Binding::new(&law.args, &[&p]).unwrap();
        assert_eq!(eval_law(&law, &b), Ok(true));
    }

    #[test]
    fn missing_field_is_an_error_not_a_panic() {
        let law = first_law(
            "\
sort Module { layer: Int }
relation depends_on(importer: Module, imported: Module)
law depends_on(importer, imported) { imported.layer <= importer.layer }",
        );
        let importer = inst("a", &[("layer", 2)]);
        let imported = inst("b", &[]); // missing `layer`
        let b = Binding::new(&law.args, &[&importer, &imported]).unwrap();
        let err = eval_law(&law, &b).unwrap_err();
        assert!(err.contains("no field `layer`"), "got: {err}");
    }

    #[test]
    fn ordering_on_identity_is_an_error() {
        let law = first_law(
            "\
sort P
relation r(a: P, b: P)
law r(a, b) { a <= b }",
        );
        let x = inst("x", &[]);
        let y = inst("y", &[]);
        let b = Binding::new(&law.args, &[&x, &y]).unwrap();
        let err = eval_law(&law, &b).unwrap_err();
        assert!(err.contains("needs integers"), "got: {err}");
    }
}
