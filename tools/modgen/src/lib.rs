//! modgen: validates `cairn.module/v1-draft` modules and emits the per-surface artefacts.
//!
//! The sibling of the firmware's `enginegen`, on purpose: same language, same shape, same
//! rule that it enforces what a JSON Schema cannot. A module set is data; this is the one
//! program that reads it, so it is also the one place a mistake in a manifest can be
//! caught before it reaches a server, a dashboard, a phone or a car.

pub mod emit;
pub mod env;
pub mod manifest;

use std::path::{Path, PathBuf};

pub use manifest::{Env, Module};

/// Loads every module under `dir` (one subdirectory each), in id order.
pub fn load_dir(dir: &Path) -> Result<Vec<Module>, String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    let mut out = Vec::new();
    for d in entries {
        out.push(load(&d)?);
    }
    out.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(out)
}

/// Loads one module directory: its manifest, its queries file if the manifest names one,
/// and its content hash.
pub fn load(dir: &Path) -> Result<Module, String> {
    let name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mpath = dir.join("module.yaml");
    let text = std::fs::read_to_string(&mpath).map_err(|e| format!("{}: {e}", mpath.display()))?;
    let manifest =
        manifest::parse_manifest(&text).map_err(|e| format!("{}: {e}", mpath.display()))?;

    let queries = match &manifest.queries {
        Some(rel) => {
            let qpath = dir.join(rel);
            let qtext =
                std::fs::read_to_string(&qpath).map_err(|e| format!("{}: {e}", qpath.display()))?;
            Some(manifest::parse_queries(&qtext).map_err(|e| format!("{}: {e}", qpath.display()))?)
        }
        None => None,
    };

    // Hashed from the declarations the manifest names, so it must be parsed first.
    let sha256 = manifest::hash_declarations(dir, &manifest)?;
    let m = Module {
        manifest,
        dir: dir.to_path_buf(),
        queries,
        sha256,
    };

    // The directory name is part of the module's identity, so it is checked here rather
    // than left to a caller that may not know where the manifest came from.
    let errs = manifest::check(&m.manifest, &name, m.queries.as_ref(), &Env::default());
    if !errs.is_empty() {
        return Err(format!("{}:\n  - {}", dir.display(), errs.join("\n  - ")));
    }
    Ok(m)
}

/// Validates a loaded set against the contracts, and against each other.
pub fn validate(mods: &[Module], env: &Env) -> Vec<String> {
    let mut errs = Vec::new();
    for m in mods {
        let name = m
            .dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        for e in manifest::check(&m.manifest, &name, m.queries.as_ref(), env) {
            errs.push(format!("{}: {e}", m.manifest.id));
        }
    }
    errs.extend(manifest::check_set(mods));
    errs
}

/// The module-set identity of `contracts/module/v1/spec.md` §7: SHA-256 of
/// `cairn.module-set/v1-draft\n` then, for each module in id order,
/// `<id>\n<version>\n<hex hash>\n`.
///
/// This exists so a derived store can prove which module set produced it. A store whose
/// contents depend on an unrecorded module set cannot prove it reproduces, and that is
/// the project's fifth invariant.
pub fn set_identity(mods: &[Module]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"cairn.module-set/v1-draft\n");
    let mut sorted: Vec<&Module> = mods.iter().collect();
    sorted.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    for m in sorted {
        h.update(m.manifest.id.as_bytes());
        h.update(b"\n");
        h.update(m.manifest.version.to_string().as_bytes());
        h.update(b"\n");
        h.update(manifest::hex64(&m.sha256).as_bytes());
        h.update(b"\n");
    }
    h.finalize().into()
}

/// `modules=<id>@<version>/<hash16>,... set=<hash16>`, the one-line form a consumer logs
/// and reports. Mirrors `enginegen`'s engine identity string exactly, for the same reason.
pub fn identity_string(mods: &[Module]) -> String {
    let mut sorted: Vec<&Module> = mods.iter().collect();
    sorted.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    let list: Vec<String> = sorted
        .iter()
        .map(|m| {
            format!(
                "{}@{}/{}",
                m.manifest.id,
                m.manifest.version,
                manifest::hex16(&m.sha256)
            )
        })
        .collect();
    format!(
        "modules={} set={}",
        list.join(","),
        manifest::hex16(&set_identity(mods))
    )
}

/// Resolves a selection: `all`, or a comma-separated list of ids. Strict, like
/// `enginegen`'s: an unknown id or an empty selection is an error, never a silent subset,
/// because a build that quietly dropped a module would be a build nobody asked for.
pub fn select<'a>(mods: &'a [Module], sel: &str) -> Result<Vec<&'a Module>, String> {
    if sel.trim() == "all" {
        if mods.is_empty() {
            return Err("there are no modules to select".into());
        }
        return Ok(mods.iter().collect());
    }
    let wanted: Vec<&str> = sel
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if wanted.is_empty() {
        return Err("an empty selection selects nothing; pass `all` or a list of ids".into());
    }
    let mut out = Vec::new();
    for w in &wanted {
        match mods.iter().find(|m| m.manifest.id == *w) {
            Some(m) => out.push(m),
            None => {
                let known: Vec<&str> = mods.iter().map(|m| m.manifest.id.as_str()).collect();
                return Err(format!("no module {w:?}; known: {}", known.join(", ")));
            }
        }
    }
    out.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(out)
}
