//! The `cairn.module/v1-draft` manifest: parsing and the checks a JSON Schema cannot
//! state (`contracts/module/v1/spec.md` §3).
//!
//! Deliberately written from the spec rather than ported from the contracts repository's
//! Go `modulecheck`. That makes this the second implementation the contract's release
//! gate asks for; two readers written from one head agree on that head's mistakes.
//!
//! The schema states structure. This states meaning: that a required column exists, that
//! exactly one module owns a derived column, that derivation order has no cycle, that a
//! metric reads a view its own module creates, and that a query's parameters match its
//! SQL in both directions.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const MANIFEST_SCHEMA: &str = "cairn.module/v1-draft";
pub const QUERIES_SCHEMA: &str = "cairn.module-queries/v1-draft";

/// A manifest as written. `deny_unknown_fields` throughout: a manifest is not a place to
/// stash data, so an extra key is an error and not an extension point (spec §2).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub id: String,
    pub version: u32,
    pub name: String,
    /// Validated by hand rather than as a serde enum: serde's "unknown variant" message
    /// is an implementation detail, and a contract vector matches the rejection text. The
    /// documented wording belongs to the project, not to the deserializer.
    pub status: String,
    pub sources: Vec<String>,
    #[serde(default)]
    pub requires: Option<Requires>,
    #[serde(default)]
    pub derives: Vec<Derive>,
    #[serde(default)]
    pub metrics: Vec<Metric>,
    #[serde(default)]
    pub views: Vec<String>,
    #[serde(default)]
    pub queries: Option<String>,
    #[serde(default)]
    pub ui: Option<Ui>,
    #[serde(default)]
    pub ios: Option<Ios>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Stub,
    Derived,
    Verified,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Stub => "stub",
            Status::Derived => "derived",
            Status::Verified => "verified",
        }
    }

    pub fn parse(s: &str) -> Option<Status> {
        match s {
            "stub" => Some(Status::Stub),
            "derived" => Some(Status::Derived),
            "verified" => Some(Status::Verified),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    #[serde(default)]
    pub store: Vec<String>,
    #[serde(default)]
    pub engine_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derive {
    pub column: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub expr: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub sample: Sample,
    pub source_view: String,
    pub source_column: String,
    #[serde(default)]
    pub trip_insight: Option<TripInsight>,
    #[serde(default)]
    pub dashboard_highlight: bool,
    #[serde(default)]
    pub ios_gauge: Option<IosGauge>,
}

/// What one observation is. A closed set on purpose: it decides how `v_metric_samples`
/// counts, and "enough observations to judge" means nothing if a module can invent its
/// own unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sample {
    Sample,
    Pull,
    Boot,
    Trip,
}

impl Sample {
    pub fn as_str(self) -> &'static str {
        match self {
            Sample::Sample => "sample",
            Sample::Pull => "pull",
            Sample::Boot => "boot",
            Sample::Trip => "trip",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TripInsight {
    pub label: String,
    pub agg: Agg,
    pub precision: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Agg {
    Min,
    Max,
    Avg,
    Median,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IosGauge {
    pub field: String,
    pub label: String,
    pub unit: String,
    pub precision: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    pub route: String,
    pub nav: Nav,
    pub vehicle_scope: VehicleScope,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nav {
    pub label: String,
    pub group: NavGroup,
    pub order: u16,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavGroup {
    Everyday,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VehicleScope {
    All,
    Single,
    None,
}

impl VehicleScope {
    pub fn as_str(self) -> &'static str {
        match self {
            VehicleScope::All => "all",
            VehicleScope::Single => "single",
            VehicleScope::None => "none",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ios {
    #[serde(default)]
    pub trip_section: Option<TripSection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TripSection {
    pub label: String,
    pub order: u16,
}

/// A module's named, parameterised queries.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueriesFile {
    pub schema: String,
    pub module: String,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub params: Vec<Param>,
    pub sql: String,
    #[serde(default)]
    pub age_threshold_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ParamType,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    VehicleId,
    BootId,
    Date,
    Int,
    Number,
    String,
}

/// A loaded module: its manifest, where it came from, its queries if it has any, and the
/// content hash that goes into the module-set identity.
#[derive(Debug, Clone)]
pub struct Module {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub queries: Option<QueriesFile>,
    pub sha256: [u8; 32],
}

/// What a manifest is validated against: the store's columns and the engine's capture
/// fields, both read from the pinned contracts.
#[derive(Debug, Default)]
pub struct Env {
    pub store_columns: BTreeSet<String>,
    pub store_objects: BTreeSet<String>,
    pub engine_fields: BTreeSet<String>,
}

const DUCK_TYPES: &[&str] = &[
    "BOOLEAN",
    "TINYINT",
    "SMALLINT",
    "INTEGER",
    "BIGINT",
    "UTINYINT",
    "USMALLINT",
    "UINTEGER",
    "UBIGINT",
    "FLOAT",
    "DOUBLE",
    "VARCHAR",
    "DATE",
    "TIMESTAMP",
];

fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 31
        && s.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_snake(s: &str, max: usize) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= max
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

fn is_column_ref(s: &str) -> bool {
    match s.split_once('.') {
        Some((t, c)) => is_snake(t, 64) && is_snake(c, 64),
        None => false,
    }
}

/// Refuses anything that could reach outside the module's own directory.
fn check_rel_path(p: &str, what: &str, errs: &mut Vec<String>) {
    if p.is_empty() {
        errs.push(format!("{what}: path is empty"));
        return;
    }
    if p.starts_with('/') {
        errs.push(format!(
            "{what} {p:?}: path must be relative to the module directory"
        ));
        return;
    }
    if p.split('/')
        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        errs.push(format!(
            "{what} {p:?}: path must not leave the module directory"
        ));
    }
}

/// Blanks single-quoted SQL string literals so a semicolon or a `$` inside one is not
/// mistaken for a statement separator or a placeholder. A doubled quote escapes.
fn strip_sql_strings(sql: &str) -> String {
    let b = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut inside = false;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\'' {
            if inside && i + 1 < b.len() && b[i + 1] == b'\'' {
                out.push_str("  ");
                i += 2;
                continue;
            }
            inside = !inside;
            out.push(' ');
            i += 1;
            continue;
        }
        if inside {
            out.push(' ');
        } else {
            out.push(b[i] as char);
        }
        i += 1;
    }
    out
}

/// The `$name` placeholders a statement reads. Matches more than a valid parameter name
/// so a wrongly cased placeholder is reported as undeclared rather than not seen.
fn placeholders(sql: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let b = sql.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'$' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
            j += 1;
        }
        if j > start && (b[start].is_ascii_alphabetic() || b[start] == b'_') {
            out.insert(sql[start..j].to_string());
        }
        i = j.max(i + 1);
    }
    out
}

/// Validates one manifest. `dir_name` is the directory the manifest was loaded from; the
/// id must equal it, or a module can be selected under a name that is not in it.
pub fn check(
    m: &Manifest,
    dir_name: &str,
    queries: Option<&QueriesFile>,
    env: &Env,
) -> Vec<String> {
    let mut errs = Vec::new();

    if m.schema != MANIFEST_SCHEMA {
        errs.push(format!(
            "schema {:?} is not {:?}",
            m.schema, MANIFEST_SCHEMA
        ));
    }
    if !is_id(&m.id) {
        errs.push(format!(
            "id {:?} must be lowercase alphanumeric with single hyphens, at most 31 characters",
            m.id
        ));
    } else if !dir_name.is_empty() && m.id != dir_name {
        errs.push(format!(
            "id {:?} does not match the directory name {dir_name:?}",
            m.id
        ));
    }
    if m.version < 1 || m.version > 65535 {
        errs.push(format!("version {} must be between 1 and 65535", m.version));
    }
    if m.name.trim().is_empty() {
        errs.push("a module needs a name".into());
    }
    if Status::parse(&m.status).is_none() {
        errs.push(format!(
            "status {:?} is not stub, derived or verified",
            m.status
        ));
    }
    if m.sources.is_empty() {
        errs.push(
            "sources must name at least one source: a claim without one is not reviewable".into(),
        );
    }
    for (i, s) in m.sources.iter().enumerate() {
        if s.trim().is_empty() {
            errs.push(format!("sources[{i}] is empty"));
        }
    }

    // A stub names the module and claims nothing. Checked before the content rules so the
    // message is the useful one.
    if m.manifest_claims_something() {
        let what = if !m.derives.is_empty() {
            format!("{} derived column(s)", m.derives.len())
        } else if !m.metrics.is_empty() {
            format!("{} metric(s)", m.metrics.len())
        } else {
            let n = m.requires.as_ref().map_or(0, |r| r.engine_fields.len());
            format!("{n} engine field(s)")
        };
        errs.push(format!("a stub claims nothing, but this one states {what}"));
    }

    if let Some(r) = &m.requires {
        for col in &r.store {
            if !is_column_ref(col) {
                errs.push(format!(
                    "requires.store {col:?} must be table.column in lowercase"
                ));
                continue;
            }
            // A column another module derives is legitimate; with one manifest in hand the
            // module's own derivations are the only other source.
            let own = m.derives.iter().any(|d| &d.column == col);
            if !env.store_columns.is_empty() && !env.store_columns.contains(col) && !own {
                errs.push(format!(
                    "requires.store {col:?} is not in store/v1 and no module derives it"
                ));
            }
        }
        for f in &r.engine_fields {
            if !env.engine_fields.is_empty() && !env.engine_fields.contains(f) {
                errs.push(format!(
                    "requires.engine_fields {f:?} is not an engine/v1 capture field"
                ));
            }
        }
    }

    let mut owned: BTreeSet<&str> = BTreeSet::new();
    for (i, d) in m.derives.iter().enumerate() {
        if !is_column_ref(&d.column) {
            errs.push(format!(
                "derives[{i}].column {:?} must be table.column in lowercase",
                d.column
            ));
        } else if !owned.insert(&d.column) {
            errs.push(format!(
                "derives[{i}]: {:?} is already owned; exactly one derivation owns a column",
                d.column
            ));
        } else if !env.store_objects.is_empty() {
            let table = d.column.split('.').next().unwrap_or_default();
            if !env.store_objects.contains(table) {
                errs.push(format!(
                    "derives[{i}]: {:?} is on {table:?}, which store/v1 does not have",
                    d.column
                ));
            }
        }
        if !DUCK_TYPES.contains(&d.ty.as_str()) {
            errs.push(format!(
                "derives[{i}].type {:?} is not a store/v1 type ({})",
                d.ty,
                DUCK_TYPES.join(", ")
            ));
        }
        if d.expr.trim().is_empty() {
            errs.push(format!("derives[{i}].expr is empty"));
        }
        if strip_sql_strings(&d.expr).contains(';') {
            errs.push(format!(
                "derives[{i}].expr holds a semicolon: a derivation is one expression"
            ));
        }
    }

    for (i, v) in m.views.iter().enumerate() {
        check_rel_path(v, &format!("views[{i}]"), &mut errs);
    }

    let own_prefix = format!("v_{}_", m.id.replace('-', "_"));
    let mut seen_metric: BTreeSet<&str> = BTreeSet::new();
    for (i, mt) in m.metrics.iter().enumerate() {
        let where_ = format!("metrics[{i}] ({})", mt.key);
        if !is_snake(&mt.key, 40) {
            errs.push(format!(
                "metrics[{i}].key {:?} must be lowercase snake_case",
                mt.key
            ));
        } else if !seen_metric.insert(&mt.key) {
            errs.push(format!("metrics[{i}].key {:?} is repeated", mt.key));
        }
        if mt.label.trim().is_empty() {
            errs.push(format!("{where_} needs a label"));
        }
        if !mt.source_view.starts_with("v_") || !is_snake(&mt.source_view, 64) {
            errs.push(format!(
                "{where_}: source_view {:?} must be named v_<something>",
                mt.source_view
            ));
        } else {
            // A metric must read a view this module creates, so the metric list and the
            // views cannot drift apart. store/v1's own views are the exception: a module
            // taking over an existing page reads them before it owns them.
            let grandfathered = env.store_objects.contains(&mt.source_view);
            let declares = !m.views.is_empty() && mt.source_view.starts_with(&own_prefix);
            if !grandfathered && !declares {
                errs.push(format!(
                    "{where_}: source_view {:?} is neither in store/v1 nor a view this module declares (name it {own_prefix}* in one of views[])",
                    mt.source_view
                ));
            }
        }
        if mt.source_column.is_empty() {
            errs.push(format!("{where_} needs a source_column"));
        }
        if let Some(ti) = &mt.trip_insight {
            if ti.label.trim().is_empty() {
                errs.push(format!("{where_}: trip_insight needs a label"));
            }
            if ti.precision > 4 {
                errs.push(format!(
                    "{where_}: trip_insight.precision {} is above 4",
                    ti.precision
                ));
            }
        }
        if let Some(g) = &mt.ios_gauge {
            if g.precision > 4 {
                errs.push(format!(
                    "{where_}: ios_gauge.precision {} is above 4",
                    g.precision
                ));
            }
            if g.field.is_empty() || !g.field.bytes().all(|b| b.is_ascii_alphanumeric()) {
                errs.push(format!(
                    "{where_}: ios_gauge.field {:?} must be alphanumeric",
                    g.field
                ));
            }
        }
    }

    if let Some(q) = &m.queries {
        check_rel_path(q, "queries", &mut errs);
    }

    if let Some(ui) = &m.ui {
        let r = ui.route.as_bytes();
        let ok = r.first() == Some(&b'/')
            && r.len() > 1
            && r[1..]
                .iter()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-' || *c == b'/');
        if !ok {
            errs.push(format!(
                "ui.route {:?} must be a lowercase path starting with /",
                ui.route
            ));
        }
        if ui.nav.label.trim().is_empty() {
            errs.push("ui.nav needs a label".into());
        }
        if ui.nav.order > 999 {
            errs.push(format!("ui.nav.order {} is above 999", ui.nav.order));
        }
        if let Some(icon) = &ui.nav.icon {
            if icon.is_empty()
                || !icon
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                errs.push(format!(
                    "ui.nav.icon {icon:?} must be lowercase with hyphens"
                ));
            }
        }
    }

    // Whether a named queries file exists on disk is the loader's business (see
    // lib::load); a manifest on its own -- a contract vector, say -- has no directory to
    // resolve it against.
    if let Some(q) = queries {
        errs.extend(check_queries(q, &m.id));
    }

    errs
}

impl Manifest {
    /// Whether a `stub` has overreached. Separate so the message can name what it claimed.
    fn manifest_claims_something(&self) -> bool {
        Status::parse(&self.status) == Some(Status::Stub)
            && (!self.derives.is_empty()
                || !self.metrics.is_empty()
                || self
                    .requires
                    .as_ref()
                    .is_some_and(|r| !r.engine_fields.is_empty()))
    }
}

/// Validates a query file. `module` is the owning manifest's id; empty skips ownership.
pub fn check_queries(q: &QueriesFile, module: &str) -> Vec<String> {
    let mut errs = Vec::new();

    if q.schema != QUERIES_SCHEMA {
        errs.push(format!(
            "queries schema {:?} is not {:?}",
            q.schema, QUERIES_SCHEMA
        ));
    }
    if !is_id(&q.module) {
        errs.push(format!(
            "queries module {:?} must be lowercase with single hyphens",
            q.module
        ));
    } else if !module.is_empty() && q.module != module {
        errs.push(format!(
            "queries module {:?} does not match the manifest's module {module:?}",
            q.module
        ));
    }
    if q.queries.is_empty() {
        errs.push("queries must hold at least one query".into());
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (i, qq) in q.queries.iter().enumerate() {
        let where_ = if qq.name.is_empty() {
            format!("queries[{i}]")
        } else {
            format!("queries[{i}] ({})", qq.name)
        };
        let name_ok = !qq.name.is_empty()
            && qq.name.len() <= 40
            && qq.name.as_bytes()[0].is_ascii_lowercase()
            && qq
                .name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !name_ok {
            errs.push(format!(
                "{where_}: name {:?} must be lowercase, digits and hyphens",
                qq.name
            ));
        } else if !seen.insert(&qq.name) {
            errs.push(format!("{where_}: duplicate query name {:?}", qq.name));
        }

        let mut declared: BTreeSet<&str> = BTreeSet::new();
        for (j, p) in qq.params.iter().enumerate() {
            if !is_snake(&p.name, 32) {
                errs.push(format!(
                    "{where_}: params[{j}] name {:?} must be lowercase snake_case",
                    p.name
                ));
            } else if !declared.insert(&p.name) {
                errs.push(format!(
                    "{where_}: params[{j}] name {:?} is repeated",
                    p.name
                ));
            }
        }

        let sql = qq.sql.trim();
        if sql.is_empty() {
            errs.push(format!("{where_}: sql is empty"));
            continue;
        }
        let upper = sql.to_ascii_uppercase();
        if !upper.starts_with("SELECT") && !upper.starts_with("WITH") {
            errs.push(format!(
                "{where_}: a query must be a SELECT (or a WITH leading to one); nothing a module declares may write to the store"
            ));
        }
        let bare = strip_sql_strings(sql);
        if bare.contains(';') {
            errs.push(format!(
                "{where_}: sql holds a semicolon outside a string; one statement per query"
            ));
        }

        // Both directions. A query may neither read an unbound parameter nor silently
        // ignore an argument a caller passed.
        let used = placeholders(&bare);
        for u in &used {
            if !declared.contains(u.as_str()) {
                errs.push(format!(
                    "{where_}: sql reads ${u}, which is not declared in params"
                ));
            }
        }
        for d in &declared {
            if !used.contains(*d) {
                errs.push(format!(
                    "{where_}: params declares {d}, which does not appear as ${d} in the sql"
                ));
            }
        }
    }
    errs
}

/// The cross-module rules: one owner per derived column, no cycle in derivation order,
/// and no repeated id.
pub fn check_set(mods: &[Module]) -> Vec<String> {
    let mut errs = Vec::new();
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();

    for m in mods {
        if !ids.insert(&m.manifest.id) {
            errs.push(format!("two modules share the id {:?}", m.manifest.id));
        }
        for d in &m.manifest.derives {
            if let Some(prev) = owner.get(d.column.as_str()) {
                errs.push(format!(
                    "column {:?} is derived by both {prev:?} and {:?}; exactly one module owns a column",
                    d.column, m.manifest.id
                ));
                continue;
            }
            owner.insert(&d.column, &m.manifest.id);
        }
    }

    // A module that reads another's derived column runs after it. A cycle means no load
    // order exists, and the store's contents would depend on the order chosen.
    let mut deps: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for m in mods {
        let e = deps.entry(&m.manifest.id).or_default();
        if let Some(r) = &m.manifest.requires {
            for col in &r.store {
                if let Some(o) = owner.get(col.as_str()) {
                    if *o != m.manifest.id.as_str() {
                        e.insert(o);
                    }
                }
            }
        }
    }
    if let Some(cycle) = find_cycle(&deps) {
        errs.push(format!(
            "derivation order has a cycle: {}",
            cycle.join(" -> ")
        ));
    }
    errs
}

/// One cycle as a path, or none. Ids are visited in sorted order so the reported cycle is
/// the same on every run.
fn find_cycle<'a>(deps: &BTreeMap<&'a str, BTreeSet<&'a str>>) -> Option<Vec<&'a str>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        White,
        Grey,
        Black,
    }
    let mut state: BTreeMap<&str, Mark> = deps.keys().map(|k| (*k, Mark::White)).collect();
    let mut path: Vec<&str> = Vec::new();

    fn walk<'a>(
        n: &'a str,
        deps: &BTreeMap<&'a str, BTreeSet<&'a str>>,
        state: &mut BTreeMap<&'a str, Mark>,
        path: &mut Vec<&'a str>,
    ) -> Option<Vec<&'a str>> {
        state.insert(n, Mark::Grey);
        path.push(n);
        for d in deps.get(n).into_iter().flatten() {
            if !deps.contains_key(*d) {
                continue;
            }
            match state.get(*d).copied().unwrap_or(Mark::White) {
                Mark::Grey => {
                    let at = path.iter().position(|p| p == d).unwrap_or(0);
                    let mut c = path[at..].to_vec();
                    c.push(d);
                    return Some(c);
                }
                Mark::White => {
                    if let Some(c) = walk(d, deps, state, path) {
                        return Some(c);
                    }
                }
                Mark::Black => {}
            }
        }
        path.pop();
        state.insert(n, Mark::Black);
        None
    }

    for id in deps.keys() {
        if state.get(id).copied().unwrap_or(Mark::White) == Mark::White {
            path.clear();
            if let Some(c) = walk(id, deps, &mut state, &mut path) {
                return Some(c);
            }
        }
    }
    None
}

/// Parses a manifest from YAML, refusing duplicate keys and unknown fields.
///
/// `serde_yaml` rejects a duplicate mapping key, which matters: many YAML parsers keep
/// the last value silently, so a reviewer could see `status: verified` while the loader
/// reads `status: stub`.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}

pub fn parse_queries(text: &str) -> Result<QueriesFile, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}

/// SHA-256 of a module directory: every file, in sorted path order, with CRLF read as LF
/// so a checkout's line endings cannot change a module's identity.
/// SHA-256 over a module's **declarations**: its manifest, then every file the manifest
/// names (`views[]`, `queries`), sorted. Each contributes its module-relative path, a
/// `0x00` byte, its bytes with CRLF read as LF, and a `0x00` byte.
///
/// Declarations only, not the directory. A `README.md` cannot change what a derivation
/// computes or what a query returns, so it must not move the hash: the module-set identity
/// reaches the store contract digest, and a prose fix that made a rebuilt store look like
/// a different store would be a false mismatch. Invariant 5 is worth less every time it
/// cries wolf.
///
/// Needs no "except documentation" carve-out, and extends by itself — a later key whose
/// value is a path is covered the day it is added.
pub fn hash_declarations(dir: &Path, m: &Manifest) -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};

    let mut named: Vec<String> = m.views.clone();
    if let Some(q) = &m.queries {
        named.push(q.clone());
    }
    named.sort();
    named.dedup();

    let mut h = Sha256::new();
    // The manifest first, then the paths it names, so the order is fixed by the rule and
    // not by a directory walk.
    for rel in std::iter::once("module.yaml".to_string()).chain(named) {
        let path = dir.join(&rel);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        h.update(rel.as_bytes());
        h.update([0u8]);
        h.update(normalise(&bytes));
        h.update([0u8]);
    }
    Ok(h.finalize().into())
}

fn normalise(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\r' && i + 1 < b.len() && b[i + 1] == b'\n' {
            out.push(b'\n');
            i += 2;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

pub fn hex16(h: &[u8; 32]) -> String {
    h[..8].iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hex64(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
