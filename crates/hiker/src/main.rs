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

use std::process::ExitCode;

use hiker::backends::{self, EmitOptions};
use hiker::{checker, parser};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => cmd_check(&args[1..]),
        Some("gen") => cmd_gen(&args[1..]),
        _ => {
            eprintln!("usage:");
            eprintln!("  hiker check <file.tent>");
            eprintln!(
                "  hiker gen   <file.tent> [--target {}] [-o <out>] [--module <name>]",
                backends::TARGETS.join("|")
            );
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
