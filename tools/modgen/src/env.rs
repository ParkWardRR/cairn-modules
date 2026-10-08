//! Reading the pinned contracts: which store columns exist, and which capture fields an
//! engine profile may declare.
//!
//! These two checks are the reason `modgen` needs the contracts at all. Without them a
//! manifest could require a column nobody has or a field no dongle records, and the
//! failure would surface as an empty page rather than as a validation error.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::manifest::Env;

/// Resolves the contracts root the way every Cairn repository does: `CAIRN_CONTRACTS`
/// wins, so a contract and its implementation can change together on a laptop;
/// otherwise the copy `scripts/fetch-contracts.sh` pinned.
pub fn contracts_root(repo_root: &Path) -> PathBuf {
    match std::env::var_os("CAIRN_CONTRACTS") {
        Some(v) => PathBuf::from(v),
        None => repo_root.join(".contracts/contracts"),
    }
}

/// Loads the validation environment from the contracts tree.
pub fn load(contracts: &Path) -> Result<Env, String> {
    let store = contracts.join("store/v1/schema.json");
    let engine = contracts.join("engine/v1/acquisition.schema.json");
    let module = contracts.join("module/v1/module.schema.json");

    for p in [&store, &engine, &module] {
        if !p.is_file() {
            return Err(format!(
                "{} is missing: run scripts/fetch-contracts.sh, or point CAIRN_CONTRACTS at a checkout's contracts/ directory",
                p.display()
            ));
        }
    }

    let (store_objects, store_columns) = store_schema(&store)?;
    let engine_fields = enum_at(&engine, "field")?;

    // The module schema carries the capture-field enum too, so that a manifest can be
    // checked without the engine contract. Two copies of one vocabulary is exactly the
    // drift engine/v1 was created to end, so they are compared rather than trusted.
    let module_fields = enum_at(&module, "engine_field")?;
    let mut drift: Vec<String> = Vec::new();
    for f in module_fields.difference(&engine_fields) {
        drift.push(format!(
            "module/v1 lists engine field {f:?}, which engine/v1 does not"
        ));
    }
    for f in engine_fields.difference(&module_fields) {
        drift.push(format!(
            "engine/v1 has capture field {f:?}, which module/v1 does not list"
        ));
    }
    if !drift.is_empty() {
        return Err(format!(
            "the capture-field vocabulary has drifted between the two contracts:\n  - {}",
            drift.join("\n  - ")
        ));
    }

    Ok(Env {
        store_columns,
        store_objects,
        engine_fields,
    })
}

/// `$defs/<name>/enum` from a JSON Schema.
fn enum_at(path: &Path, def: &str) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let items = doc
        .get("$defs")
        .and_then(|d| d.get(def))
        .and_then(|f| f.get("enum"))
        .and_then(|e| e.as_array())
        .ok_or_else(|| format!("{}: $defs/{def} has no enum", path.display()))?;
    let out: BTreeSet<String> = items
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if out.is_empty() {
        return Err(format!("{}: $defs/{def}/enum is empty", path.display()));
    }
    Ok(out)
}

/// The objects and `table.column` names `store/v1/schema.json` pins.
fn store_schema(path: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;

    let mut objects = BTreeSet::new();
    let mut columns = BTreeSet::new();
    for group in ["tables", "views", "macros"] {
        let Some(map) = doc.get(group).and_then(|g| g.as_object()) else {
            continue;
        };
        for (name, obj) in map {
            objects.insert(name.clone());
            let Some(cols) = obj.get("columns").and_then(|c| c.as_array()) else {
                continue;
            };
            for c in cols {
                if let Some(col) = c.get("name").and_then(|n| n.as_str()) {
                    columns.insert(format!("{name}.{col}"));
                }
            }
        }
    }
    if columns.is_empty() {
        return Err(format!(
            "{}: pins no columns; the shape must have changed",
            path.display()
        ));
    }
    Ok((objects, columns))
}
