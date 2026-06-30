//! Facts — the external evidence `verify` evaluates laws against.
//!
//! hiker never reads TS/Rust/Python source. Extraction is **external**: a tool in
//! the consuming repo (`madge --json`, `depcruise`, a grep) emits a `facts.json`
//! describing the real codebase graph, and `verify` consumes it. This module is
//! the JSON model, its loader, and a type-checker that validates facts against a
//! *checked* `Spec` before evaluation — reusing the same sort/relation tables the
//! checker builds, and collecting *all* errors rather than stopping at the first.
//!
//! ```json
//! {
//!   "instances": { "Module": [ { "id": "core", "fields": { "layer": 0 } } ] },
//!   "tuples":    { "depends_on": [ ["cli", "core"] ] }
//! }
//! ```

use std::collections::HashMap;

use serde::Deserialize;

use crate::ast::*;
use crate::eval::Instance;

/// A parsed `facts.json`: instance populations per sort, and relation tuples.
#[derive(Debug, Clone, Deserialize)]
pub struct Facts {
    /// `instances[Sort]` — one object per real entity of that sort.
    pub instances: HashMap<String, Vec<FactInstance>>,
    /// `tuples[Relation]` — arrays of instance ids, positional to the relation's
    /// params. Optional: a spec may declare relations with no extracted edges.
    #[serde(default)]
    pub tuples: HashMap<String, Vec<Vec<String>>>,
}

/// One real entity: a stable id plus the integer field values laws read. Fields
/// are optional in JSON; an identity/no-field sort simply omits them.
#[derive(Debug, Clone, Deserialize)]
pub struct FactInstance {
    pub id: String,
    #[serde(default)]
    pub fields: HashMap<String, i64>,
}

impl FactInstance {
    /// Materialize the runtime `Instance` the interpreter binds over.
    pub fn to_instance(&self) -> Instance {
        Instance {
            id: self.id.clone(),
            fields: self.fields.clone(),
        }
    }
}

/// Load and parse a `facts.json` file. IO and parse errors become `Err(String)`;
/// nothing here panics.
pub fn load(path: &str) -> Result<Facts, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read facts `{path}`: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid facts JSON in `{path}`: {e}"))
}

/// Type-check facts against a checked spec. Collects every error.
///
/// Rules:
/// 1. every `instances` key names a declared sort; every `tuples` key names a
///    declared relation;
/// 2. each instance supplies every `Int` field its sort declares (extra fields
///    are ignored for v0);
/// 3. each tuple's arity equals its relation's param count, and each element is
///    an id present in the population of the corresponding param's sort.
pub fn check_facts(spec: &Spec, facts: &Facts) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    let sorts: HashMap<&str, &Sort> = spec.sorts.iter().map(|s| (s.name.as_str(), s)).collect();
    let relations: HashMap<&str, &Relation> = spec
        .relations
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();

    // Rule 1: keys name declared sorts / relations.
    for sort_name in facts.instances.keys() {
        if !sorts.contains_key(sort_name.as_str()) {
            errors.push(format!(
                "facts: `instances` names unknown sort `{sort_name}`"
            ));
        }
    }
    for rel_name in facts.tuples.keys() {
        if !relations.contains_key(rel_name.as_str()) {
            errors.push(format!(
                "facts: `tuples` names unknown relation `{rel_name}`"
            ));
        }
    }

    // Rule 2: each instance supplies every declared Int field.
    for (sort_name, insts) in &facts.instances {
        let Some(sort) = sorts.get(sort_name.as_str()) else {
            continue; // already reported by rule 1
        };
        for inst in insts {
            for field in &sort.fields {
                if matches!(field.ty, Ty::Int) && !inst.fields.contains_key(&field.name) {
                    errors.push(format!(
                        "facts: instance `{}` of sort `{sort_name}` is missing Int field `{}`",
                        inst.id, field.name
                    ));
                }
            }
        }
    }

    // Rule 3: tuple arity + each element resolves to a real instance of its sort.
    for (rel_name, tuples) in &facts.tuples {
        let Some(rel) = relations.get(rel_name.as_str()) else {
            continue; // already reported by rule 1
        };
        for (idx, tuple) in tuples.iter().enumerate() {
            if tuple.len() != rel.params.len() {
                errors.push(format!(
                    "facts: tuple {idx} of relation `{rel_name}` has arity {} but the relation takes {}",
                    tuple.len(),
                    rel.params.len()
                ));
                continue;
            }
            for (elem, param) in tuple.iter().zip(&rel.params) {
                if resolve_id(facts, &param.sort, elem).is_none() {
                    errors.push(format!(
                        "facts: tuple {idx} of relation `{rel_name}` references id `{elem}` not declared as a `{}` instance",
                        param.sort
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The instance population of a sort (empty slice if the sort has no facts).
pub fn instances_of<'a>(facts: &'a Facts, sort: &str) -> &'a [FactInstance] {
    facts.instances.get(sort).map_or(&[], Vec::as_slice)
}

/// Find an instance of `sort` by id.
pub fn resolve_id<'a>(facts: &'a Facts, sort: &str, id: &str) -> Option<&'a FactInstance> {
    instances_of(facts, sort).iter().find(|i| i.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    const SPEC: &str = "\
sort Module { layer: Int }
relation depends_on(a: Module, b: Module)";

    fn facts(json: &str) -> Facts {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn valid_facts_pass() {
        let spec = parse(SPEC).unwrap();
        let f = facts(
            r#"{
              "instances": { "Module": [
                { "id": "core", "fields": { "layer": 0 } },
                { "id": "cli",  "fields": { "layer": 2 } }
              ] },
              "tuples": { "depends_on": [ ["cli", "core"] ] }
            }"#,
        );
        assert_eq!(check_facts(&spec, &f), Ok(()));
    }

    #[test]
    fn tuple_arity_mismatch_is_reported() {
        let spec = parse(SPEC).unwrap();
        let f = facts(
            r#"{ "instances": { "Module": [ { "id": "cli", "fields": { "layer": 2 } } ] },
                 "tuples": { "depends_on": [ ["cli"] ] } }"#,
        );
        let errs = check_facts(&spec, &f).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("arity")), "got: {errs:?}");
    }

    #[test]
    fn tuple_unknown_id_is_reported() {
        let spec = parse(SPEC).unwrap();
        let f = facts(
            r#"{ "instances": { "Module": [ { "id": "cli", "fields": { "layer": 2 } } ] },
                 "tuples": { "depends_on": [ ["cli", "ghost"] ] } }"#,
        );
        let errs = check_facts(&spec, &f).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("`ghost`")), "got: {errs:?}");
    }

    #[test]
    fn instance_missing_int_field_is_reported() {
        let spec = parse(SPEC).unwrap();
        let f = facts(r#"{ "instances": { "Module": [ { "id": "core" } ] } }"#);
        let errs = check_facts(&spec, &f).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("missing Int field `layer`")),
            "got: {errs:?}"
        );
    }

    #[test]
    fn unknown_sort_and_relation_keys_are_reported() {
        let spec = parse(SPEC).unwrap();
        let f = facts(r#"{ "instances": { "Ghost": [] }, "tuples": { "no_rel": [] } }"#);
        let errs = check_facts(&spec, &f).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("unknown sort `Ghost`")));
        assert!(errs.iter().any(|e| e.contains("unknown relation `no_rel`")));
    }
}
