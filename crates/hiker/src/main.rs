//! hiker CLI — wires the four compiler stages into two commands.
//!
//!   hiker check <file.tent>
//!       Lex + parse + check. Prints a summary and exits 0 if intent compiles,
//!       or prints every error and exits 1.
//!
//!   hiker gen <file.tent> [--target rust|ts|python] [-o <out>] [--module <name>]
//!       Checks first (refuses to generate from incoherent intent), then writes
//!       the test bridge for the chosen target. `--module` is the
//!       system-under-test crate/import/module name (default: temporal).
//!       With no `-o`, output goes to `.hiker-cache/<target>/<default-name>`.
//!       `--crate` is accepted as an alias for `--module`.
//!
//!   hiker verify <file.tent> --facts <facts.json>
//!       Checks first, then evaluates the spec's laws against facts extracted
//!       from a real codebase (the JSON `--facts` file). Exits 0 if every law
//!       holds, or prints each structural violation and exits 1.

// Tests write temp files and use `.unwrap()` for brevity; production code may not
// (see the disallowed-methods ban in clippy.toml).
#![cfg_attr(test, allow(clippy::disallowed_methods))]

use std::process::ExitCode;

use hiker::backends::{self, EmitOptions};
use hiker::{checker, facts, parser, verify};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => cmd_check(&args[1..]),
        Some("gen") => cmd_gen(&args[1..]),
        Some("verify") => cmd_verify(&args[1..]),
        Some("--version" | "-V" | "version") => {
            println!("hiker {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage:");
            eprintln!("  hiker check <file.tent>");
            eprintln!(
                "  hiker gen   <file.tent> [--target {}] [-o <out>] [--module <name>]",
                backends::TARGETS.join("|")
            );
            eprintln!("  hiker verify <file.tent> --facts <facts.json>");
            ExitCode::FAILURE
        }
    }
}

/// Read + parse a file into a checked Spec. Returns the spec or exits with the
/// errors printed.
fn load_checked(path: &str) -> Result<hiker::ast::Spec, ExitCode> {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            return Err(ExitCode::FAILURE);
        }
    };
    let spec = match parser::parse(&src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("parse error: {e}");
            return Err(ExitCode::FAILURE);
        }
    };
    if let Err(errors) = checker::check(&spec) {
        eprintln!("intent does not compile ({} error(s)):", errors.len());
        for e in &errors {
            eprintln!("  {e}");
        }
        return Err(ExitCode::FAILURE);
    }
    // Non-fatal lints (e.g. a relation with no law): surface but don't fail.
    for w in checker::warnings(&spec) {
        eprintln!("  {w}");
    }
    Ok(spec)
}

fn cmd_check(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: hiker check <file.tent>");
        return ExitCode::FAILURE;
    };
    match load_checked(path) {
        Ok(spec) => {
            println!(
                "OK: {} sorts, {} relations, {} laws",
                spec.sorts.len(),
                spec.relations.len(),
                spec.laws.len()
            );
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

fn cmd_gen(args: &[String]) -> ExitCode {
    // Parse: <file> [--target <t>] [-o <out>] [--module <name>]
    let mut path: Option<&str> = None;
    let mut out: Option<String> = None;
    let mut module = "temporal".to_string();
    let mut target = "rust".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                out = args.get(i).cloned();
            }
            "--target" => {
                i += 1;
                if let Some(t) = args.get(i) {
                    target = t.clone();
                }
            }
            "--module" | "--crate" => {
                i += 1;
                if let Some(m) = args.get(i) {
                    module = m.clone();
                }
            }
            other if path.is_none() => path = Some(other),
            other => {
                eprintln!("unexpected argument `{other}`");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!(
            "usage: hiker gen <file.tent> [--target {}] [-o <out>] [--module <name>]",
            backends::TARGETS.join("|")
        );
        return ExitCode::FAILURE;
    };

    // Resolve the backend up front so we can derive a default output path.
    let Some(backend) = backends::for_target(&target) else {
        eprintln!(
            "unknown target `{target}`. known targets: {}",
            backends::TARGETS.join(", ")
        );
        return ExitCode::FAILURE;
    };

    // Default output: .hiker-cache/<target>/<backend default file name>.
    let out =
        out.unwrap_or_else(|| format!(".hiker-cache/{target}/{}", backend.default_filename()));

    let spec = match load_checked(path) {
        Ok(s) => s,
        Err(code) => return code,
    };

    let code = backend.emit(&spec, &EmitOptions { module });
    // Create the output directory (e.g. .hiker-cache/rust/) if needed.
    if let Some(parent) = std::path::Path::new(&out).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("error: cannot create `{}`: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    if let Err(e) = std::fs::write(&out, &code) {
        eprintln!("error: cannot write `{out}`: {e}");
        return ExitCode::FAILURE;
    }
    println!("wrote {out} ({} laws -> {target} tests)", spec.laws.len());
    ExitCode::SUCCESS
}

fn cmd_verify(args: &[String]) -> ExitCode {
    // Parse: <file.tent> --facts <facts.json>
    let mut path: Option<&str> = None;
    let mut facts_path: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--facts" => {
                i += 1;
                facts_path = args.get(i).map(String::as_str);
            }
            other if path.is_none() => path = Some(other),
            other => {
                eprintln!("unexpected argument `{other}`");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let (Some(path), Some(facts_path)) = (path, facts_path) else {
        eprintln!("usage: hiker verify <file.tent> --facts <facts.json>");
        return ExitCode::FAILURE;
    };

    ExitCode::from(run_verify(path, facts_path))
}

/// The verify pipeline, returning a process exit code (0 = ok, 1 = failure).
/// Split out from `cmd_verify` so it can be unit-tested without spawning a
/// process (`ExitCode` is opaque and not comparable).
fn run_verify(spec_path: &str, facts_path: &str) -> u8 {
    let Ok(spec) = load_checked(spec_path) else {
        return 1;
    };

    let facts = match facts::load(facts_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    if let Err(errors) = facts::check_facts(&spec, &facts) {
        eprintln!("facts do not match spec ({} error(s)):", errors.len());
        for e in &errors {
            eprintln!("  {e}");
        }
        return 1;
    }

    let violations = verify::verify(&spec, &facts);
    if violations.is_empty() {
        println!("OK: 0 violations across {} laws", spec.laws.len());
        0
    } else {
        eprintln!("verify FAILED: {} violation(s):", violations.len());
        for v in &violations {
            eprintln!(
                "  {} (line {}): {} \u{2014} {}",
                v.relation,
                v.law_line,
                v.args.join(", "),
                v.detail
            );
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::run_verify;

    fn write_temp(tag: &str, contents: &str) -> String {
        let mut p = std::env::temp_dir();
        p.push(format!("hiker_verify_{}_{tag}", std::process::id()));
        std::fs::write(&p, contents).unwrap();
        p.to_string_lossy().into_owned()
    }

    const SPEC: &str = "\
sort Module { layer: Int }
relation depends_on(a: Module, b: Module)
law depends_on(a, b) { b.layer <= a.layer }
";

    const INSTANCES: &str = r#""instances": { "Module": [
        { "id": "cli",  "fields": { "layer": 2 } },
        { "id": "core", "fields": { "layer": 0 } }
    ] }"#;

    #[test]
    fn verify_succeeds_on_inward_facts() {
        let spec = write_temp("ok.tent", SPEC);
        let facts = write_temp(
            "ok.json",
            &format!(r#"{{ {INSTANCES}, "tuples": {{ "depends_on": [ ["cli","core"] ] }} }}"#),
        );
        assert_eq!(run_verify(&spec, &facts), 0);
    }

    #[test]
    fn verify_fails_on_violating_edge() {
        let spec = write_temp("bad.tent", SPEC);
        // core (layer 0) depends on cli (layer 2): 2 <= 0 is false.
        let facts = write_temp(
            "bad.json",
            &format!(r#"{{ {INSTANCES}, "tuples": {{ "depends_on": [ ["core","cli"] ] }} }}"#),
        );
        assert_eq!(run_verify(&spec, &facts), 1);
    }
}
