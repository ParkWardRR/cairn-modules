//! modgen: validate a Cairn module set and emit what each surface needs.
//!
//!   modgen validate [--dir modules]            every manifest, against the contracts
//!   modgen list     [--dir modules]            what is here
//!   modgen identity [--dir modules]            the module-set identity string
//!   modgen vectors  [--contracts <dir>]        run contracts/module/v1's own vectors
//!   modgen gen --target fields|go --out <path> [--modules all|a,b]
//!
//! `vectors` is the important one for the contract: it makes this the second independent
//! implementation `contracts/module/v1/README.md` names in its release gate. There is
//! deliberately no `--update`: a checker that can rewrite the vectors it is checked
//! against is not a check.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use modgen::{emit, env, manifest};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("modgen: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "\
modgen — validate a Cairn module set and emit what each surface needs

  modgen validate [--dir modules]
  modgen list     [--dir modules]
  modgen identity [--dir modules]
  modgen vectors  [--contracts <dir>]
  modgen gen --target fields|go --out <path> [--dir modules] [--modules all|a,b] [--package name]

CAIRN_CONTRACTS overrides the pinned contracts in .contracts/contracts.";

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn repo_root() -> PathBuf {
    // The crate lives at <root>/tools/modgen.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn modules_dir(args: &[String]) -> PathBuf {
    flag(args, "--dir").map(PathBuf::from).unwrap_or_else(|| {
        let p = PathBuf::from("modules");
        if p.is_dir() {
            p
        } else {
            repo_root().join("modules")
        }
    })
}

fn contracts_dir(args: &[String]) -> PathBuf {
    flag(args, "--contracts")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::contracts_root(&repo_root()))
}

fn run(args: &[String]) -> Result<(), String> {
    match args[0].as_str() {
        "validate" => {
            let (mods, e) = load_and_env(args)?;
            let errs = modgen::validate(&mods, &e);
            if !errs.is_empty() {
                return Err(format!(
                    "{} problem(s):\n  - {}",
                    errs.len(),
                    errs.join("\n  - ")
                ));
            }
            for m in &mods {
                println!("ok  {}", emit::summary(m));
            }
            println!("{} module(s) valid", mods.len());
            Ok(())
        }
        "list" => {
            let mods = modgen::load_dir(&modules_dir(args))?;
            for m in &mods {
                println!("{}", emit::summary(m));
            }
            Ok(())
        }
        "identity" => {
            let mods = modgen::load_dir(&modules_dir(args))?;
            println!("{}", modgen::identity_string(&mods));
            Ok(())
        }
        "vectors" => vectors(&contracts_dir(args)),
        "gen" => {
            let target =
                flag(args, "--target").ok_or("gen needs --target fields or --target go")?;
            let out = flag(args, "--out").ok_or("gen needs --out <path>")?;
            let sel = flag(args, "--modules").unwrap_or_else(|| "all".into());
            let (mods, e) = load_and_env(args)?;
            let errs = modgen::validate(&mods, &e);
            if !errs.is_empty() {
                return Err(format!(
                    "refusing to generate from an invalid set:\n  - {}",
                    errs.join("\n  - ")
                ));
            }
            let chosen = modgen::select(&mods, &sel)?;
            let identity = modgen::identity_string(&mods);
            let text = match target.as_str() {
                "fields" => emit::fields(&chosen),
                "go" => {
                    let pkg = flag(args, "--package").unwrap_or_else(|| "modules".into());
                    emit::go(&chosen, &identity, &pkg)
                }
                other => {
                    return Err(format!(
                        "unknown target {other:?}; `fields` and `go` exist. The Nuxt and Swift targets land with the web and app phases, when there is a consumer to shape them"
                    ))
                }
            };
            let path = PathBuf::from(&out);
            if let Some(d) = path.parent() {
                std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
            }
            std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
            eprintln!("modgen: {identity}");
            println!(
                "wrote {} ({target}, {} module(s))",
                path.display(),
                chosen.len()
            );
            Ok(())
        }
        other => Err(format!("unknown command {other:?}\n{USAGE}")),
    }
}

fn load_and_env(args: &[String]) -> Result<(Vec<modgen::Module>, modgen::Env), String> {
    let mods = modgen::load_dir(&modules_dir(args))?;
    let e = env::load(&contracts_dir(args))?;
    Ok((mods, e))
}

/// Runs `contracts/module/v1/vectors` against this implementation.
///
/// A valid vector is a bare document. An invalid one is wrapped with the text its
/// rejection must contain, matched as a substring so wording can improve without
/// breaking a vector.
fn vectors(contracts: &Path) -> Result<(), String> {
    let dir = contracts.join("module/v1/vectors");
    if !dir.is_dir() {
        return Err(format!(
            "{} is missing: run scripts/fetch-contracts.sh, or set CAIRN_CONTRACTS",
            dir.display()
        ));
    }
    // The vectors are JSON, which is valid YAML, so one file serves both kinds of
    // implementation. Read them with the YAML parser this program uses for manifests, so
    // the vectors exercise the real path.
    let mut files = Vec::new();
    collect_json(&dir, &mut files)?;
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: no vectors", dir.display()));
    }

    let env = env::load(contracts).unwrap_or_default();
    let mut valid = 0usize;
    let mut invalid = 0usize;
    let mut failed = 0usize;

    for f in &files {
        let rel = f.strip_prefix(contracts).unwrap_or(f).display().to_string();
        let text = std::fs::read_to_string(f).map_err(|e| format!("{rel}: {e}"))?;
        let doc: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))?;

        let expect = doc.get("expect_error").and_then(|v| v.as_str());
        let dir_name = doc.get("directory").and_then(|v| v.as_str());
        let owner = doc.get("module").and_then(|v| v.as_str());

        let errs = match expect {
            None => {
                valid += 1;
                // A bare document: a manifest, or a queries file if its schema says so.
                let is_q =
                    doc.get("schema").and_then(|v| v.as_str()) == Some(manifest::QUERIES_SCHEMA);
                if is_q {
                    match manifest::parse_queries(&text) {
                        Ok(q) => manifest::check_queries(&q, ""),
                        Err(e) => vec![e],
                    }
                } else {
                    match manifest::parse_manifest(&text) {
                        // A flat vector file is not a directory, so the id-equals-directory
                        // rule cannot apply to it.
                        Ok(m) => {
                            let name = m.id.clone();
                            manifest::check(&m, &name, None, &env)
                        }
                        Err(e) => vec![e],
                    }
                }
            }
            Some(_) => {
                invalid += 1;
                if let Some(body) = doc.get("queries_file") {
                    let s = body.to_string();
                    match manifest::parse_queries(&s) {
                        Ok(q) => manifest::check_queries(&q, owner.unwrap_or_default()),
                        Err(e) => vec![e],
                    }
                } else if let Some(body) = doc.get("manifest") {
                    let s = body.to_string();
                    match manifest::parse_manifest(&s) {
                        Ok(m) => {
                            let name = dir_name.map(str::to_string).unwrap_or_else(|| m.id.clone());
                            manifest::check(&m, &name, None, &env)
                        }
                        Err(e) => vec![e],
                    }
                } else {
                    vec!["vector has expect_error but no manifest or queries_file".into()]
                }
            }
        };

        let joined = errs.join("; ");
        match expect {
            None if !joined.is_empty() => {
                failed += 1;
                println!("FAIL {rel}: should be accepted, but was rejected: {joined}");
            }
            Some(want) if joined.is_empty() => {
                failed += 1;
                println!("FAIL {rel}: should be rejected mentioning {want:?}, but was accepted");
            }
            Some(want) if !joined.contains(want) => {
                failed += 1;
                println!("FAIL {rel}: was rejected as {joined:?}, which does not mention {want:?}");
            }
            _ => {}
        }
    }

    println!(
        "modgen vectors: {} vectors ({valid} valid, {invalid} invalid), {failed} failed",
        files.len()
    );
    if failed > 0 {
        return Err(format!("{failed} vector(s) failed"));
    }
    Ok(())
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.is_dir() {
            collect_json(&p, out)?;
        } else if p.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(p);
        }
    }
    Ok(())
}
