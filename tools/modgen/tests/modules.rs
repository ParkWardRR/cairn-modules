//! The shipped modules are valid; the contract's vectors hold; the cross-module rules
//! reject what they must; identity is stable and sensitive to the right things.

use std::path::{Path, PathBuf};

use modgen::manifest::{self, Derive, Env, Manifest, Module, Status};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn modules_dir() -> PathBuf {
    root().join("modules")
}

fn contracts() -> PathBuf {
    modgen::env::contracts_root(&root())
}

fn env() -> Env {
    modgen::env::load(&contracts()).unwrap_or_else(|e| {
        panic!("the pinned contracts must be present: {e}");
    })
}

#[test]
fn shipped_modules_are_valid() {
    let mods = modgen::load_dir(&modules_dir()).expect("modules/ must load");
    let errs = modgen::validate(&mods, &env());
    assert!(errs.is_empty(), "modules/ does not validate: {errs:?}");
    assert!(!mods.is_empty(), "there are no modules at all");
}

/// The case a validator is most likely to get wrong: a module may be nothing but a
/// manifest -- no metric, no derivation, no engine field, not even a page. `places` will
/// be close to it, and a validator that only accepts the full shape would reject it.
///
/// Built from a fixture rather than from whichever shipped module happens to be a stub:
/// `boost` was one until it took over its derivation, and a property test that moves with
/// the modules is a property test that stops testing the property.
#[test]
fn a_declaration_only_module_is_valid_and_generates_nothing() {
    let tmp = std::env::temp_dir().join(format!("modgen-stub-{}", std::process::id()));
    let dir = tmp.join("quiet");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("module.yaml"),
        format!(
            "schema: {}\nid: quiet\nversion: 1\nname: Claims nothing\nstatus: stub\nsources: [a fixture]\n",
            manifest::MANIFEST_SCHEMA
        ),
    )
    .unwrap();

    let mods = modgen::load_dir(&tmp).expect("a declaration-only module must load");
    assert_eq!(mods.len(), 1);
    let m = &mods[0];
    assert_eq!(Status::parse(&m.manifest.status), Some(Status::Stub));
    assert!(m.manifest.derives.is_empty() && m.manifest.metrics.is_empty());

    let chosen: Vec<&Module> = mods.iter().collect();
    let fields = modgen::emit::fields(&chosen);
    assert!(
        fields.contains("(none:"),
        "a module requiring no capture field must emit an empty field set, got:\n{fields}"
    );
    // The Go artefact is still emitted: the server needs to know the module exists and
    // what its digest is, even when it claims nothing.
    let go = modgen::emit::go(&chosen, "modules=quiet@1/x set=y", "modules");
    assert!(go.contains("ID:      \"quiet\""));
    assert!(!go.contains("Derives:"), "a stub must emit no derivations");
    assert!(!go.contains("Metrics:"), "a stub must emit no metrics");

    std::fs::remove_dir_all(&tmp).ok();
}

/// The shipped boost module took over a grandfathered column, so it must emit the
/// derivation and the capture fields it needs -- the other half of the same mechanism.
#[test]
fn boost_declares_its_derivation_and_its_capture_fields() {
    let mods = modgen::load_dir(&modules_dir()).unwrap();
    let boost = mods
        .iter()
        .find(|m| m.manifest.id == "boost")
        .expect("boost must exist");

    let d = match boost.manifest.derives.as_slice() {
        [only] => only,
        other => panic!("boost should own exactly one column, found {}", other.len()),
    };
    assert_eq!(d.column, "boost.boost_psi");
    assert_eq!(d.ty, "DOUBLE");

    // The two mistakes an earlier draft made, pinned here so the manifest cannot drift
    // back into them. Saturation is excluded by the views that read the column, never by
    // the column; and the reference treats a barometric 0 as a reading, not as absence.
    assert!(
        !d.expr.contains("map_kpa <") && !d.expr.contains("baro_kpa >"),
        "the derivation must not exclude saturation or treat baro 0 as absent: {}",
        d.expr
    );
    assert!(d.expr.contains("IS NOT NULL"), "{}", d.expr);

    let chosen: Vec<&Module> = vec![boost];
    let fields = modgen::emit::fields(&chosen);
    for want in ["map_kpa", "baro_kpa", "rpm", "throttle_pct"] {
        assert!(
            fields.contains(want),
            "{want} missing from the field set:\n{fields}"
        );
    }
    let go = modgen::emit::go(&chosen, "x", "modules");
    assert!(
        go.contains("Derives:"),
        "the Go artefact must carry the derivation"
    );
}

/// The contract's own vectors, run against this implementation. `modgen` is the second
/// implementation named in contracts/module/v1/README.md's release gate; the binary's
/// `vectors` subcommand is the same check, and this keeps `cargo test` covering it.
#[test]
fn contract_vectors_hold() {
    let dir = contracts().join("module/v1/vectors");
    assert!(dir.is_dir(), "{} is missing", dir.display());
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.sort();
    assert!(files.len() >= 20, "only {} vectors", files.len());

    let env = env();
    let mut valid = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
        let expect = doc.get("expect_error").and_then(|v| v.as_str());
        let rel = f.strip_prefix(&dir).unwrap().display().to_string();

        let errs: Vec<String> = match expect {
            None => {
                valid += 1;
                if doc.get("schema").and_then(|v| v.as_str()) == Some(manifest::QUERIES_SCHEMA) {
                    match manifest::parse_queries(&text) {
                        Ok(q) => manifest::check_queries(&q, ""),
                        Err(e) => vec![e],
                    }
                } else {
                    match manifest::parse_manifest(&text) {
                        Ok(m) => {
                            let name = m.id.clone();
                            manifest::check(&m, &name, None, &env)
                        }
                        Err(e) => vec![e],
                    }
                }
            }
            Some(_) => {
                let owner = doc
                    .get("module")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let dir_name = doc.get("directory").and_then(|v| v.as_str());
                if let Some(b) = doc.get("queries_file") {
                    match manifest::parse_queries(&b.to_string()) {
                        Ok(q) => manifest::check_queries(&q, owner),
                        Err(e) => vec![e],
                    }
                } else {
                    let b = doc.get("manifest").expect("a wrapped vector needs a body");
                    match manifest::parse_manifest(&b.to_string()) {
                        Ok(m) => {
                            let name = dir_name.map(str::to_string).unwrap_or_else(|| m.id.clone());
                            manifest::check(&m, &name, None, &env)
                        }
                        Err(e) => vec![e],
                    }
                }
            }
        };

        let joined = errs.join("; ");
        match expect {
            None => assert!(joined.is_empty(), "{rel} should be accepted: {joined}"),
            Some(want) => {
                assert!(
                    !joined.is_empty(),
                    "{rel} should be rejected mentioning {want:?}"
                );
                assert!(
                    joined.contains(want),
                    "{rel} was rejected as {joined:?}, which does not mention {want:?}"
                );
            }
        }
    }
    assert!(
        valid > 0,
        "no valid vectors; a checker only shown bad input is untested"
    );
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(p);
        }
    }
}

/// The capture-field vocabulary must be identical in engine/v1 and module/v1. engine/v1
/// exists because two copies of one vocabulary drifted apart, and env::load compares them
/// rather than trusting either.
#[test]
fn the_capture_field_vocabulary_has_not_drifted() {
    modgen::env::load(&contracts()).expect("the two contracts' field enums must agree");
}

fn fixture(id: &str, derives: Vec<Derive>, requires: Vec<&str>) -> Module {
    let yaml = format!(
        "schema: {}\nid: {id}\nversion: 1\nname: {id}\nstatus: derived\nsources: [a fixture]\n",
        manifest::MANIFEST_SCHEMA
    );
    let mut m: Manifest = manifest::parse_manifest(&yaml).unwrap();
    m.derives = derives;
    if !requires.is_empty() {
        m.requires = Some(manifest::Requires {
            store: requires.iter().map(|s| s.to_string()).collect(),
            engine_fields: vec![],
        });
    }
    Module {
        manifest: m,
        dir: PathBuf::from(id),
        queries: None,
        sha256: [0u8; 32],
    }
}

fn derive(col: &str) -> Derive {
    Derive {
        column: col.to_string(),
        ty: "DOUBLE".into(),
        expr: "1".into(),
        note: None,
    }
}

#[test]
fn two_modules_cannot_own_one_derived_column() {
    let errs = manifest::check_set(&[
        fixture("alpha", vec![derive("boost.boost_psi")], vec![]),
        fixture("beta", vec![derive("boost.boost_psi")], vec![]),
    ]);
    assert!(
        errs.iter().any(|e| e.contains("exactly one module owns")),
        "two owners were accepted: {errs:?}"
    );
}

#[test]
fn a_derivation_cycle_is_rejected() {
    let errs = manifest::check_set(&[
        fixture("alpha", vec![derive("boost.a_col")], vec!["boost.b_col"]),
        fixture("beta", vec![derive("boost.b_col")], vec!["boost.a_col"]),
    ]);
    assert!(
        errs.iter().any(|e| e.contains("cycle")),
        "a cycle was accepted: {errs:?}"
    );
}

/// A chain must still be allowed, or a module could never read another's derived column —
/// and fuel-economy genuinely needs the lambda that fuel-mixture derives.
#[test]
fn a_chain_is_not_a_cycle() {
    let errs = manifest::check_set(&[
        fixture("alpha", vec![derive("boost.a_col")], vec![]),
        fixture("beta", vec![derive("boost.b_col")], vec!["boost.a_col"]),
        fixture("gamma", vec![], vec!["boost.b_col"]),
    ]);
    assert!(
        errs.is_empty(),
        "a three-module chain was rejected: {errs:?}"
    );
}

#[test]
fn a_module_may_read_its_own_derived_column() {
    let errs = manifest::check_set(&[fixture(
        "alpha",
        vec![derive("boost.a_col")],
        vec!["boost.a_col"],
    )]);
    assert!(
        errs.is_empty(),
        "self-reference was treated as a cycle: {errs:?}"
    );
}

#[test]
fn duplicate_ids_are_rejected() {
    let errs = manifest::check_set(&[
        fixture("alpha", vec![], vec![]),
        fixture("alpha", vec![], vec![]),
    ]);
    assert!(errs.iter().any(|e| e.contains("share the id")), "{errs:?}");
}

/// A duplicate mapping key must be refused. Many YAML parsers keep the last value
/// silently, so a reviewer could see `status: verified` while the loader reads
/// `status: stub`. This is the one contract vector that must be YAML, and the reason.
#[test]
fn a_duplicate_key_is_refused() {
    let yaml = format!(
        "schema: {}\nid: boost\nversion: 1\nname: Boost\nstatus: verified\nstatus: stub\nsources: [a]\n",
        manifest::MANIFEST_SCHEMA
    );
    let err = manifest::parse_manifest(&yaml).expect_err("a duplicate key must be refused");
    assert!(
        err.contains("duplicate"),
        "refused for the wrong reason: {err}"
    );
}

#[test]
fn an_unknown_field_is_refused() {
    let yaml = format!(
        "schema: {}\nid: boost\nversion: 1\nname: Boost\nstatus: stub\nsources: [a]\ncache_ttl_s: 30\n",
        manifest::MANIFEST_SCHEMA
    );
    let err = manifest::parse_manifest(&yaml).expect_err("an unknown field must be refused");
    assert!(
        err.contains("unknown field"),
        "refused for the wrong reason: {err}"
    );
}

/// Identity must be stable across runs and must move when a module's content moves — it
/// is what lets a derived store prove which module set produced it.
#[test]
fn identity_is_stable_and_content_sensitive() {
    let mods = modgen::load_dir(&modules_dir()).unwrap();
    let first = modgen::identity_string(&mods);
    for _ in 0..4 {
        let again = modgen::load_dir(&modules_dir()).unwrap();
        assert_eq!(
            first,
            modgen::identity_string(&again),
            "identity is not stable"
        );
    }
    assert!(first.starts_with("modules="), "unexpected shape: {first}");
    assert!(first.contains(" set="), "unexpected shape: {first}");

    // A changed version changes the set identity, even with the same file hashes.
    let mut bumped = mods.clone();
    bumped[0].manifest.version += 1;
    assert_ne!(
        modgen::set_identity(&mods),
        modgen::set_identity(&bumped),
        "a version bump must change the set identity"
    );

    // And so does a changed file digest.
    let mut rehashed = mods.clone();
    rehashed[0].sha256[0] ^= 0xff;
    assert_ne!(
        modgen::set_identity(&mods),
        modgen::set_identity(&rehashed),
        "a content change must change the set identity"
    );
}

/// The rule that module/v1 §7 was corrected to: a module's hash covers its declarations,
/// not its directory. Editing prose must NOT move it — the module-set identity reaches the
/// store contract digest, and a README fix that made a rebuilt store look like a different
/// store would be a false mismatch. Editing the manifest, or a file the manifest names,
/// must move it.
#[test]
fn the_hash_covers_declarations_and_not_prose() {
    let tmp = std::env::temp_dir().join(format!("modgen-hash-{}", std::process::id()));
    let dir = tmp.join("alpha");
    std::fs::create_dir_all(dir.join("store")).unwrap();

    let manifest = format!(
        "schema: {}\nid: alpha\nversion: 1\nname: Alpha\nstatus: derived\nsources: [a]\nviews: [store/views.sql]\n",
        manifest::MANIFEST_SCHEMA
    );
    std::fs::write(dir.join("module.yaml"), &manifest).unwrap();
    std::fs::write(
        dir.join("store/views.sql"),
        "CREATE VIEW v_alpha_x AS SELECT 1;\n",
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "# alpha\n\nFirst draft.\n").unwrap();

    let read = || {
        let text = std::fs::read_to_string(dir.join("module.yaml")).unwrap();
        let m = manifest::parse_manifest(&text).unwrap();
        manifest::hash_declarations(&dir, &m).unwrap()
    };

    let before = read();

    // Prose: must not move the hash.
    std::fs::write(dir.join("README.md"), "# alpha\n\nRewritten entirely.\n").unwrap();
    assert_eq!(before, read(), "editing README.md changed the module hash");

    // A file nobody declared: must not move it either.
    std::fs::write(dir.join("scratch.txt"), "notes to self").unwrap();
    assert_eq!(before, read(), "an undeclared file changed the module hash");

    // A file the manifest names: must move it.
    std::fs::write(
        dir.join("store/views.sql"),
        "CREATE VIEW v_alpha_x AS SELECT 2;\n",
    )
    .unwrap();
    let after_sql = read();
    assert_ne!(
        before, after_sql,
        "editing a declared view did not change the hash"
    );

    // The manifest itself: must move it.
    std::fs::write(
        dir.join("module.yaml"),
        manifest.replace("version: 1", "version: 2"),
    )
    .unwrap();
    assert_ne!(
        after_sql,
        read(),
        "editing the manifest did not change the hash"
    );

    // A named file that is missing is an error, not a silently skipped one: a manifest
    // claiming a view that is not there would otherwise hash as if it had none.
    std::fs::remove_file(dir.join("store/views.sql")).unwrap();
    let text = std::fs::read_to_string(dir.join("module.yaml")).unwrap();
    let m = manifest::parse_manifest(&text).unwrap();
    assert!(
        manifest::hash_declarations(&dir, &m).is_err(),
        "a missing declared file was skipped instead of refused"
    );

    std::fs::remove_dir_all(&tmp).ok();
}

/// A selection is strict: an unknown id or an empty list is an error, never a silent
/// subset, because a build that quietly dropped a module is a build nobody asked for.
#[test]
fn selection_is_strict() {
    let mods = modgen::load_dir(&modules_dir()).unwrap();
    assert_eq!(modgen::select(&mods, "all").unwrap().len(), mods.len());
    assert_eq!(modgen::select(&mods, "boost").unwrap().len(), 1);
    assert!(modgen::select(&mods, "bmw-nope").is_err());
    assert!(modgen::select(&mods, "").is_err());
    assert!(modgen::select(&mods, "boost,nope").is_err());
}

/// A query may neither read an unbound parameter nor silently ignore one a caller passed,
/// and a literal must not be mistaken for a placeholder or a statement separator.
#[test]
fn query_parameters_are_checked_both_ways() {
    let base = |sql: &str, params: &str| {
        format!(
            "schema: {}\nmodule: alpha\nqueries:\n  - name: q\n{params}    sql: \"{sql}\"\n",
            manifest::QUERIES_SCHEMA
        )
    };
    let one = "    params:\n      - { name: vehicle_id, type: vehicle_id }\n";

    let q = manifest::parse_queries(&base("SELECT 1 WHERE a = $vehicle_id", "")).unwrap();
    let errs = manifest::check_queries(&q, "alpha");
    assert!(errs.iter().any(|e| e.contains("not declared")), "{errs:?}");

    let q = manifest::parse_queries(&base("SELECT 1", one)).unwrap();
    let errs = manifest::check_queries(&q, "alpha");
    assert!(
        errs.iter().any(|e| e.contains("does not appear")),
        "{errs:?}"
    );

    // A semicolon and a $ inside string literals are not a separator or a placeholder.
    let q = manifest::parse_queries(&base(
        "SELECT 'a;b' AS s, '$nope' AS l FROM boost WHERE vehicle_id = $vehicle_id",
        one,
    ))
    .unwrap();
    assert!(
        manifest::check_queries(&q, "alpha").is_empty(),
        "a literal was mistaken for syntax"
    );

    // Nothing a module declares may write to the store.
    for bad in [
        "DELETE FROM boost",
        "UPDATE boost SET x = 1",
        "ATTACH 'x.db'",
    ] {
        let q = manifest::parse_queries(&base(bad, "")).unwrap();
        assert!(
            !manifest::check_queries(&q, "alpha").is_empty(),
            "{bad:?} was accepted"
        );
    }
    // A WITH leading to a SELECT is fine: v_pulls-style queries are common-table
    // expressions.
    let q =
        manifest::parse_queries(&base("WITH x AS (SELECT 1 AS n) SELECT n FROM x", "")).unwrap();
    assert!(
        manifest::check_queries(&q, "alpha").is_empty(),
        "a WITH query was rejected"
    );
}

#[test]
fn a_stub_claims_nothing() {
    let yaml = format!(
        "schema: {}\nid: alpha\nversion: 1\nname: Alpha\nstatus: stub\nsources: [a]\n",
        manifest::MANIFEST_SCHEMA
    );
    let mut m: Manifest = manifest::parse_manifest(&yaml).unwrap();
    assert!(manifest::check(&m, "alpha", None, &Env::default()).is_empty());

    m.derives = vec![derive("boost.x")];
    let errs = manifest::check(&m, "alpha", None, &Env::default());
    assert!(
        errs.iter().any(|e| e.contains("a stub claims nothing")),
        "a stub with a derivation was accepted: {errs:?}"
    );
}

#[test]
fn a_required_column_must_exist_in_store_v1() {
    let e = env();
    assert!(
        e.store_columns.contains("boost.map_kpa"),
        "store/v1 should pin boost.map_kpa"
    );
    assert!(!e.store_columns.contains("boost.turbo_rpm"));

    let yaml = format!(
        "schema: {}\nid: alpha\nversion: 1\nname: Alpha\nstatus: derived\nsources: [a]\nrequires:\n  store: [boost.turbo_rpm]\n",
        manifest::MANIFEST_SCHEMA
    );
    let m: Manifest = manifest::parse_manifest(&yaml).unwrap();
    let errs = manifest::check(&m, "alpha", None, &e);
    assert!(
        errs.iter().any(|x| x.contains("store/v1")),
        "an unknown column was accepted: {errs:?}"
    );
}

#[test]
fn an_id_must_match_its_directory() {
    let yaml = format!(
        "schema: {}\nid: boost\nversion: 1\nname: Boost\nstatus: stub\nsources: [a]\n",
        manifest::MANIFEST_SCHEMA
    );
    let m: Manifest = manifest::parse_manifest(&yaml).unwrap();
    let errs = manifest::check(&m, "turbo", None, &Env::default());
    assert!(
        errs.iter().any(|e| e.contains("directory name")),
        "a mismatched directory was accepted: {errs:?}"
    );
}
