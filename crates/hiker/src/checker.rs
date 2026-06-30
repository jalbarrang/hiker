//! Checker — stage 3 of the compiler. This is what "intent compiles" means.
//!
//! The parser proved the file is *shaped* like a spec. The checker proves the
//! spec is *coherent*: every name resolves, every relation is well-typed, and
//! every comparison makes sense. If this passes, your architectural intent is
//! at least internally consistent — it cannot quietly contradict itself.
//!
//! This is the stage that defeats the "collapse" bug from the videos:
//!   * A relation's parameters are typed, so a law's arguments have known sorts.
//!   * `p.t0` is rejected if `p` is a TemporalPoint (it has no `t0`).
//!   * `a.media <= b.media` is rejected because `<=` needs integers.
//!   * `p.media == i.t0` is rejected (comparing a MediaItem with an Int).
//!
//! We collect *all* errors rather than stopping at the first, so one run tells
//! you everything that is wrong.

use std::collections::HashMap;

use crate::ast::*;

/// The type of an expression inside a law: either the built-in integer, or an
/// instance of some sort.
#[derive(Debug, Clone, PartialEq)]
enum ExprTy {
    Int,
    Sort(String),
}

/// Check a spec for coherence. `Ok(())` means intent compiles.
pub fn check(spec: &Spec) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Index sorts and relations by name for quick lookup. Flag duplicates.
    let mut sorts: HashMap<&str, &Sort> = HashMap::new();
    for s in &spec.sorts {
        if sorts.insert(s.name.as_str(), s).is_some() {
            errors.push(format!("line {}: sort `{}` declared twice", s.line, s.name));
        }
    }
    let mut relations: HashMap<&str, &Relation> = HashMap::new();
    for r in &spec.relations {
        if relations.insert(r.name.as_str(), r).is_some() {
            errors.push(format!(
                "line {}: relation `{}` declared twice",
                r.line, r.name
            ));
        }
    }

    // Rule 0: every sort field whose type is another sort must reference a
    // declared sort.
    for s in &spec.sorts {
        for f in &s.fields {
            if let Ty::Sort(name) = &f.ty {
                if !sorts.contains_key(name.as_str()) {
                    errors.push(format!(
                        "line {}: field `{}.{}` has unknown type `{}`",
                        s.line, s.name, f.name, name
                    ));
                }
            }
        }
    }

    // Rule 1: every relation parameter sort must be declared.
    for r in &spec.relations {
        for p in &r.params {
            if !sorts.contains_key(p.sort.as_str()) {
                errors.push(format!(
                    "line {}: relation `{}` parameter `{}` has unknown sort `{}`",
                    r.line, r.name, p.name, p.sort
                ));
            }
        }
    }

    // Rules 2-5: check each law against its relation.
    for law in &spec.laws {
        check_law(law, &sorts, &relations, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_law(
    law: &Law,
    sorts: &HashMap<&str, &Sort>,
    relations: &HashMap<&str, &Relation>,
    errors: &mut Vec<String>,
) {
    // Rule 2: the law's relation must exist, with matching arity.
    let relation = match relations.get(law.relation.as_str()) {
        Some(r) => *r,
        None => {
            errors.push(format!(
                "line {}: law refers to unknown relation `{}`",
                law.line, law.relation
            ));
            return;
        }
    };
    // A `forbidden` relation means "no fact may match"; it has no behavior to
    // constrain, so attaching a law to it is meaningless — reject it.
    if relation.forbidden {
        errors.push(format!(
            "line {}: law on `forbidden` relation `{}` — forbidden relations allow no facts, so a law is meaningless",
            law.line, law.relation
        ));
        return;
    }
    if law.args.len() != relation.params.len() {
        errors.push(format!(
            "line {}: law `{}` has {} argument(s) but the relation takes {}",
            law.line,
            law.relation,
            law.args.len(),
            relation.params.len()
        ));
        return;
    }

    // Bind each law argument to the sort declared in the relation's signature.
    // THIS is the contract that pins meaning: `p` is a TemporalPoint because the
    // relation said so, not because of how it's used.
    let mut arg_sort: HashMap<&str, &str> = HashMap::new();
    for (arg, param) in law.args.iter().zip(&relation.params) {
        arg_sort.insert(arg.as_str(), param.sort.as_str());
    }

    // An empty body constrains nothing: the generated test's oracle would be
    // `true`, so the implementation could do anything and still pass. That is a
    // silent enforcement hole, so reject it loudly.
    if law.clauses.is_empty() {
        errors.push(format!(
            "line {}: law `{}` has an empty body (constrains nothing)",
            law.line, law.relation
        ));
    }

    // Check every clause. An implication type-checks both of its comparisons.
    for clause in &law.clauses {
        match clause {
            Clause::Compare(p) => check_pred(p, &arg_sort, sorts, errors),
            Clause::Implies { ante, cons, .. } => {
                check_pred(ante, &arg_sort, sorts, errors);
                check_pred(cons, &arg_sort, sorts, errors);
            }
        }
    }
}

/// Type-check a single comparison.
fn check_pred(
    pred: &Pred,
    arg_sort: &HashMap<&str, &str>,
    sorts: &HashMap<&str, &Sort>,
    errors: &mut Vec<String>,
) {
    let lhs = type_of(&pred.lhs, arg_sort, sorts, pred.line, errors);
    let rhs = type_of(&pred.rhs, arg_sort, sorts, pred.line, errors);
    // If a name failed to resolve, the error is already recorded; skip.
    let (Some(lhs), Some(rhs)) = (lhs, rhs) else {
        return;
    };

    match pred.op {
        // Ordering operators require integers on both sides.
        CmpOp::Le | CmpOp::Lt | CmpOp::Ge | CmpOp::Gt => {
            if lhs != ExprTy::Int || rhs != ExprTy::Int {
                errors.push(format!(
                    "line {}: comparison `{}` needs integers, got {} and {}",
                    pred.line,
                    op_str(pred.op),
                    ty_str(&lhs),
                    ty_str(&rhs),
                ));
            }
        }
        // Equality requires both sides to be the *same* type.
        CmpOp::Eq => {
            if lhs != rhs {
                errors.push(format!(
                    "line {}: `==` compares mismatched types {} and {}",
                    pred.line,
                    ty_str(&lhs),
                    ty_str(&rhs),
                ));
            }
        }
    }
}

/// Work out the type of an expression, recording an error and returning `None`
/// if a name does not resolve.
fn type_of(
    expr: &Expr,
    arg_sort: &HashMap<&str, &str>,
    sorts: &HashMap<&str, &Sort>,
    line: usize,
    errors: &mut Vec<String>,
) -> Option<ExprTy> {
    match expr {
        Expr::Int(_) => Some(ExprTy::Int),
        Expr::Arg(name) => match arg_sort.get(name.as_str()) {
            Some(sort) => Some(ExprTy::Sort((*sort).to_string())),
            None => {
                errors.push(format!("line {line}: unknown argument `{name}`"));
                None
            }
        },
        Expr::Field { arg, field } => {
            let sort_name = match arg_sort.get(arg.as_str()) {
                Some(s) => *s,
                None => {
                    errors.push(format!("line {line}: unknown argument `{arg}`"));
                    return None;
                }
            };
            // The sort must exist (it will, since relation params were checked)
            // and must actually have this field. This rejects `p.t0` on a point.
            let sort = sorts.get(sort_name)?;
            match sort.fields.iter().find(|f| f.name == *field) {
                Some(f) => Some(match &f.ty {
                    Ty::Int => ExprTy::Int,
                    Ty::Sort(s) => ExprTy::Sort(s.clone()),
                }),
                None => {
                    errors.push(format!(
                        "line {line}: sort `{sort_name}` has no field `{field}` (in `{arg}.{field}`)"
                    ));
                    None
                }
            }
        }
    }
}

/// Non-fatal lints: things that compile but probably aren't what you meant.
///
/// Currently one rule: a relation with no law is *declared intent that is never
/// enforced* (no test is generated for it). hiker's whole point is that drift
/// fails loudly, so we surface this rather than let it pass silently. It is a
/// warning, not an error, because declaring a relation before writing its law
/// is a legitimate work-in-progress state.
pub fn warnings(spec: &Spec) -> Vec<String> {
    let mut out = Vec::new();
    for r in &spec.relations {
        // A `forbidden` relation is *defined* to have no law — its enforcement is
        // "zero matching facts", checked by verify — so don't warn about it.
        if r.forbidden {
            continue;
        }
        if !spec.laws.iter().any(|law| law.relation == r.name) {
            out.push(format!(
                "warning: relation `{}` has no law \u{2014} declared intent is not enforced",
                r.name
            ));
        }
    }
    out
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

fn ty_str(t: &ExprTy) -> String {
    match t {
        ExprTy::Int => "Int".to_string(),
        ExprTy::Sort(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn check_src(src: &str) -> Result<(), Vec<String>> {
        check(&parse(src).unwrap())
    }

    #[test]
    fn real_spec_compiles() {
        let src = include_str!("../../../.hiker/temporal.tent");
        assert!(check_src(src).is_ok());
    }

    #[test]
    fn rejects_unknown_param_sort() {
        let errs = check_src("relation r(a: Ghost)").unwrap_err();
        assert!(errs.iter().any(|e| e.contains("unknown sort `Ghost`")));
    }

    #[test]
    fn rejects_unknown_relation_in_law() {
        let errs = check_src("law nope(a) { }").unwrap_err();
        assert!(errs.iter().any(|e| e.contains("unknown relation `nope`")));
    }

    #[test]
    fn rejects_arity_mismatch() {
        let src = "\
sort P
relation r(a: P, b: P)
law r(a) { }";
        let errs = check_src(src).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("argument(s)")));
    }

    #[test]
    fn rejects_collapsing_point_into_interval() {
        // The video's bug, as a type error: `p` is a point, it has no `t0`.
        let src = "\
sort MediaItem
sort TemporalPoint { media: MediaItem, t: Int }
sort TemporalInterval { media: MediaItem, t0: Int, t1: Int }
relation point_in_interval(p: TemporalPoint, i: TemporalInterval)
law point_in_interval(p, i) {
  p.t0 <= i.t1
}";
        let errs = check_src(src).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("has no field `t0`")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn rejects_empty_law_body() {
        let src = "\
sort P
relation r(a: P)
law r(a) { }";
        let errs = check_src(src).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("empty body")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn rejects_law_on_forbidden_relation() {
        let src = "\
sort File
forbidden relation fs(f: File)
law fs(f) { }";
        let errs = check_src(src).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("forbidden")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn no_unlawed_warning_for_forbidden_relation() {
        let spec = parse("sort File\nforbidden relation fs(f: File)").unwrap();
        assert!(warnings(&spec).is_empty(), "got: {:?}", warnings(&spec));
    }

    #[test]
    fn warns_on_unlawed_relation() {
        let spec = parse("sort P\nrelation r(a: P, b: P)").unwrap();
        let warns = warnings(&spec);
        assert!(
            warns.iter().any(|w| w.contains("relation `r` has no law")),
            "got: {warns:?}"
        );
    }

    #[test]
    fn real_spec_has_no_warnings() {
        let spec = parse(include_str!("../../../.hiker/temporal.tent")).unwrap();
        assert!(warnings(&spec).is_empty(), "got: {:?}", warnings(&spec));
    }

    #[test]
    fn rejects_ordering_on_non_int() {
        let src = "\
sort MediaItem
sort P { media: MediaItem }
relation r(a: P, b: P)
law r(a, b) {
  a.media <= b.media
}";
        let errs = check_src(src).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("needs integers")));
    }

    #[test]
    fn rejects_eq_type_mismatch() {
        let src = "\
sort MediaItem
sort P { media: MediaItem, t: Int }
relation r(a: P)
law r(a) {
  a.media == a.t
}";
        let errs = check_src(src).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("mismatched types")));
    }
}
