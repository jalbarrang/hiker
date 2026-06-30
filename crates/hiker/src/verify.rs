//! Verify — conformance of a spec's laws against extracted facts.
//!
//! This is hiker's third safety net. `check` proves intent compiles; `gen` emits
//! property tests over *random* inputs (behavioral laws). `verify` evaluates the
//! *same laws* over a *finite population of facts* a consuming repo extracted —
//! catching **structural** violations (dependency direction, "no fs in adapters")
//! that a generated test cannot see because they describe the codebase graph, not
//! a callable function.
//!
//! Quantification is inferred, no grammar change:
//! - **Tuple-driven** — if facts supply tuples for a law's relation (e.g.
//!   `depends_on` edges), each tuple is one binding to check.
//! - **Cross-product** — otherwise, enumerate the cartesian product of the
//!   instance populations of the law's parameter sorts (the common case is pairs
//!   over one sort, e.g. `single_source(a, b)`). Capped to avoid blow-up.

use crate::ast::*;
use crate::eval::{eval_law, Binding, Instance};
use crate::facts::{self, Facts};

/// One law failure: which law, on which line, the instance ids involved, and a
/// human-readable detail (the violated law, or the fact gap that broke eval).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub relation: String,
    pub law_line: usize,
    pub args: Vec<String>,
    pub detail: String,
}

/// Cross-product safety valve: past this many combinations we refuse and tell
/// the user to supply explicit tuples instead.
const MAX_COMBINATIONS: usize = 10_000;

/// Evaluate every law in `spec` against `facts`, returning all violations.
///
/// Assumes `facts` already passed `facts::check_facts` (so ids resolve and arity
/// matches); it still never panics on a gap — an unresolved binding becomes a
/// violation rather than a crash.
pub fn verify(spec: &Spec, facts: &Facts) -> Vec<Violation> {
    let mut out = Vec::new();

    // Forbidden relations carry no law: their rule is "zero matching facts", so
    // any extracted tuple is a violation.
    for relation in &spec.relations {
        if !relation.forbidden {
            continue;
        }
        if let Some(tuples) = facts.tuples.get(&relation.name) {
            for tuple in tuples {
                out.push(Violation {
                    relation: relation.name.clone(),
                    law_line: relation.line,
                    args: tuple.clone(),
                    detail: "forbidden relation has a matching fact".to_string(),
                });
            }
        }
    }

    for law in &spec.laws {
        let Some(relation) = spec.relations.iter().find(|r| r.name == law.relation) else {
            continue; // checker guarantees the relation exists; be defensive
        };
        match facts.tuples.get(&law.relation) {
            Some(tuples) => verify_tuples(law, relation, tuples, facts, &mut out),
            None => verify_cross_product(law, relation, facts, &mut out),
        }
    }
    out
}

/// Tuple-driven: each extracted tuple is one binding.
fn verify_tuples(
    law: &Law,
    relation: &Relation,
    tuples: &[Vec<String>],
    facts: &Facts,
    out: &mut Vec<Violation>,
) {
    for tuple in tuples {
        let mut insts: Vec<Instance> = Vec::with_capacity(tuple.len());
        let mut resolved = true;
        for (id, param) in tuple.iter().zip(&relation.params) {
            match facts::resolve_id(facts, &param.sort, id) {
                Some(fi) => insts.push(fi.to_instance()),
                None => {
                    out.push(Violation {
                        relation: law.relation.clone(),
                        law_line: law.line,
                        args: tuple.clone(),
                        detail: format!("tuple references unknown id `{id}`"),
                    });
                    resolved = false;
                    break;
                }
            }
        }
        if resolved {
            check_combo(law, tuple.clone(), &insts, out);
        }
    }
}

/// Cross-product: enumerate the cartesian product of each param sort's population.
fn verify_cross_product(law: &Law, relation: &Relation, facts: &Facts, out: &mut Vec<Violation>) {
    let pops: Vec<&[facts::FactInstance]> = relation
        .params
        .iter()
        .map(|p| facts::instances_of(facts, &p.sort))
        .collect();

    // Checked product: overflow or exceeding the cap both refuse, with guidance.
    let total = pops
        .iter()
        .try_fold(1usize, |acc, p| acc.checked_mul(p.len()));
    match total {
        Some(n) if n <= MAX_COMBINATIONS => {}
        _ => {
            out.push(Violation {
                relation: law.relation.clone(),
                law_line: law.line,
                args: Vec::new(),
                detail: format!(
                    "cross-product population for `{}` exceeds cap {MAX_COMBINATIONS}; supply explicit tuples in facts",
                    law.relation
                ),
            });
            return;
        }
    }

    for combo in cartesian(&pops) {
        let insts: Vec<Instance> = combo.iter().map(|fi| fi.to_instance()).collect();
        let ids: Vec<String> = combo.iter().map(|fi| fi.id.clone()).collect();
        check_combo(law, ids, &insts, out);
    }
}

/// Bind one combination, evaluate the law, and push a violation if it fails.
fn check_combo(law: &Law, ids: Vec<String>, insts: &[Instance], out: &mut Vec<Violation>) {
    let refs: Vec<&Instance> = insts.iter().collect();
    let detail = match Binding::new(&law.args, &refs) {
        Ok(b) => match eval_law(law, &b) {
            Ok(true) => return,
            Ok(false) => "law violated by these facts".to_string(),
            Err(e) => e,
        },
        Err(e) => e,
    };
    out.push(Violation {
        relation: law.relation.clone(),
        law_line: law.line,
        args: ids,
        detail,
    });
}

/// Cartesian product of the populations, as rows of borrowed instances. An empty
/// population yields no rows; an empty `pops` yields one empty row.
fn cartesian<'a>(pops: &[&'a [facts::FactInstance]]) -> Vec<Vec<&'a facts::FactInstance>> {
    let mut rows: Vec<Vec<&facts::FactInstance>> = vec![Vec::new()];
    for pop in pops {
        let mut next = Vec::new();
        for prefix in &rows {
            for item in *pop {
                let mut row = prefix.clone();
                row.push(item);
                next.push(row);
            }
        }
        rows = next;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn facts(json: &str) -> Facts {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn dependency_direction_passes_inward_and_flags_the_outward_edge() {
        let spec = parse(
            "\
sort Module { layer: Int }
relation depends_on(a: Module, b: Module)
law depends_on(a, b) { b.layer <= a.layer }",
        )
        .unwrap();

        let inward = facts(
            r#"{ "instances": { "Module": [
                   { "id": "m0", "fields": { "layer": 0 } },
                   { "id": "m1", "fields": { "layer": 1 } },
                   { "id": "m2", "fields": { "layer": 2 } }
                 ] },
                 "tuples": { "depends_on": [ ["m2","m1"], ["m1","m0"] ] } }"#,
        );
        assert_eq!(verify(&spec, &inward), Vec::new());

        let outward = facts(
            r#"{ "instances": { "Module": [
                   { "id": "m0", "fields": { "layer": 0 } },
                   { "id": "m2", "fields": { "layer": 2 } }
                 ] },
                 "tuples": { "depends_on": [ ["m2","m0"], ["m0","m2"] ] } }"#,
        );
        let v = verify(&spec, &outward);
        assert_eq!(v.len(), 1, "exactly one violation, got: {v:?}");
        assert_eq!(v[0].args, vec!["m0".to_string(), "m2".to_string()]);
        assert_eq!(v[0].relation, "depends_on");
    }

    #[test]
    fn single_source_cross_product_flags_divergent_results() {
        let spec = parse(
            "\
sort Core { usecase: Int, result: Int }
relation single_source(a: Core, b: Core)
law single_source(a, b) { a.usecase == b.usecase => a.result == b.result }",
        )
        .unwrap();

        // Same usecase, different result → at least one ordered pair violates.
        let diverge = facts(
            r#"{ "instances": { "Core": [
                   { "id": "x", "fields": { "usecase": 1, "result": 10 } },
                   { "id": "y", "fields": { "usecase": 1, "result": 20 } }
                 ] } }"#,
        );
        let v = verify(&spec, &diverge);
        assert!(!v.is_empty(), "expected a violation");
        assert!(v.iter().all(|viol| viol.relation == "single_source"));
        assert!(
            v.iter()
                .any(|viol| viol.args.contains(&"x".to_string())
                    && viol.args.contains(&"y".to_string())),
            "violation should name x and y, got: {v:?}"
        );

        // Same usecase, same result → conformant.
        let agree = facts(
            r#"{ "instances": { "Core": [
                   { "id": "x", "fields": { "usecase": 1, "result": 10 } },
                   { "id": "y", "fields": { "usecase": 1, "result": 10 } }
                 ] } }"#,
        );
        assert_eq!(verify(&spec, &agree), Vec::new());
    }

    #[test]
    fn forbidden_relation_flags_each_matching_fact() {
        let spec = parse(
            "\
sort File
forbidden relation fs_import_in_adapter(f: File)",
        )
        .unwrap();

        let with_fact = facts(
            r#"{ "instances": {}, "tuples": { "fs_import_in_adapter": [ ["routers/a.ts"] ] } }"#,
        );
        let v = verify(&spec, &with_fact);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].relation, "fs_import_in_adapter");
        assert!(v[0].detail.contains("forbidden"), "got: {:?}", v[0]);

        let no_fact = facts(r#"{ "instances": {} }"#);
        assert_eq!(verify(&spec, &no_fact), Vec::new());
    }
}
