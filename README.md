<!-- cairn-nav:start -->
<p align="center"><b>Cairn is a family of six repositories.</b> Each builds, tests and releases on its own; they agree through the shared <a href="https://github.com/ParkWardRR/cairn-driving-log-selfhosted/tree/main/contracts">contracts</a>, and they share one <a href="https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/ROADMAP.md">roadmap</a>.</p>

| Part | Repository | What it does | Stack | Docs | Issues | CI |
|---|---|---|---|---|---|---|
| Front door | [cairn-driving-log-selfhosted](https://github.com/ParkWardRR/cairn-driving-log-selfhosted) | Docs, roadmap, shared protocol contracts | Markdown · Go tools | [docs](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/tree/main/docs) | [issues](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/issues) | [CI](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/actions) |
| Dongle | [cairn-esp32-device-firmware](https://github.com/ParkWardRR/cairn-esp32-device-firmware) | In-car recorder: OBD-II, GNSS, IMU to encrypted SD bundles | C++ · C · Rust | [docs](https://github.com/ParkWardRR/cairn-esp32-device-firmware/tree/main/docs) | [issues](https://github.com/ParkWardRR/cairn-esp32-device-firmware/issues) | [CI](https://github.com/ParkWardRR/cairn-esp32-device-firmware/actions) |
| Phone | [cairn-ios-companion-app](https://github.com/ParkWardRR/cairn-ios-companion-app) | BLE relay, GPS assist, server client | Swift · SwiftUI | [docs](https://github.com/ParkWardRR/cairn-ios-companion-app/tree/main/docs) | [issues](https://github.com/ParkWardRR/cairn-ios-companion-app/issues) | [CI](https://github.com/ParkWardRR/cairn-ios-companion-app/actions) |
| Server | [cairn-vehicle-server](https://github.com/ParkWardRR/cairn-vehicle-server) | Verifies, decrypts, stores; serves app and dashboard | Go | [docs](https://github.com/ParkWardRR/cairn-vehicle-server/tree/main/docs) | [issues](https://github.com/ParkWardRR/cairn-vehicle-server/issues) | [CI](https://github.com/ParkWardRR/cairn-vehicle-server/actions) |
| Dashboard | [cairn-vehicle-web-dashboard](https://github.com/ParkWardRR/cairn-vehicle-web-dashboard) | Browser UI: trips, places, engine, health | Nuxt · TypeScript | [docs](https://github.com/ParkWardRR/cairn-vehicle-web-dashboard/tree/main/docs) | [issues](https://github.com/ParkWardRR/cairn-vehicle-web-dashboard/issues) | [CI](https://github.com/ParkWardRR/cairn-vehicle-web-dashboard/actions) |
| Modules | **[cairn-modules](https://github.com/ParkWardRR/cairn-modules)** ◀ you are here | Interpretation, separated from the logging core: one package per module | YAML · Rust | [readme](https://github.com/ParkWardRR/cairn-modules#readme) | [issues](https://github.com/ParkWardRR/cairn-modules/issues) | [CI](https://github.com/ParkWardRR/cairn-modules/actions) |

<sub>Shared: [Roadmap](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/ROADMAP.md) · [Module system plan](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/docs/module-system-plan.md) · [module/v1 contract](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/tree/main/contracts/module/v1) · [Architecture](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/docs/architecture.md) · [Threat model](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/docs/threat-model.md)</sub>
<!-- cairn-nav:end -->
<h1 align="center">Cairn modules</h1>
<p align="center"><strong>Interpretation, separated from the logging core. One module is one package carrying its dongle, server, web and app parts together.</strong></p>

<p align="center">
  <a href="https://github.com/ParkWardRR/cairn-modules/actions"><img src="https://img.shields.io/github/actions/workflow/status/ParkWardRR/cairn-modules/ci.yml?style=flat-square&label=CI" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Blue_Oak_1.0.0-2E86C1?style=flat-square" alt="Blue Oak Model License 1.0.0"></a>
  <img src="https://img.shields.io/badge/modgen-Rust-2E86C1?style=flat-square" alt="modgen, in Rust">
  <img src="https://img.shields.io/badge/contracts-v0.4.0-2E86C1?style=flat-square" alt="contracts-v0.4.0">
  <img src="https://img.shields.io/badge/module%2Fv1-draft-E67E22?style=flat-square" alt="module/v1 is draft">
  <img src="https://img.shields.io/badge/modules-1_of_6-E67E22?style=flat-square" alt="One module of six">
</p>

---

Cairn's core is **GPS logging management**: capture a drive, seal it, get it off the card
against a verified receipt, decode it, keep it. A **module** is *interpretation* — what a
reading means — and this repository is where modules live.

Boost, fuel economy, fuel trims, driving style, the speedometer check and place kinds are
each a module; the logging core is not.

> **One module is live, and five are not.** `boost` now owns the `boost.boost_psi`
> derivation in the server's analytical store, as of M3 — so this is no longer a repository
> that nothing reads. What it does **not** yet carry anywhere is a module **view**, a metric
> or a named query, and the other five modules do not exist. See [MODULES.md](MODULES.md)
> for what each module claims today, and the project's single
> [roadmap](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/ROADMAP.md#m1m7--the-module-system--in-progress)
> for M4 onward.

**Contents:**
[Why it exists](#why-it-exists) ·
[Not the retired plugin system](#this-is-not-the-plugin-system-that-was-retired) ·
[Layout](#layout) ·
[modgen](#modgen) ·
[CI](#ci) ·
[Contracts](#contracts) ·
[What is not here yet](#what-is-not-here-yet) ·
[Identity and the pin](#identity-runs-ahead-of-the-pin) ·
[Licence](#licence)

## Why it exists

The project has carried an acceptance test since the rebuild and never met it:

> **Adaptability** — a new vehicle, a new place kind or a new metric can be added by
> configuration or a documented extension point, not by editing core code.

Today, adding a metric means editing a core view, a core decode path, a core nav array, a
core scope table and a generated inventory of 88 SQL strings. Six of the dashboard's
twelve pages are interpretation living inside the core. The clearest sign is already in
the analytical store: `v_metric_samples`, `v_health_stats` and `v_tune_effect` are generic
machinery wrapped around a hard-coded
`VALUES ('boost_psi'), ('lambda'), ('ltft_pct'), ('stft_pct')`. **That literal is what a
module contributes.**

## This is not the plugin system that was retired

Arbitrary user plugins were retired on 2026-10-05 with the MoonBit/WASM system and remain
deferred. A module is a different thing:

| Retired plugins | Modules |
|---|---|
| Shipped executable code (WASM) to a host | Declarations only: manifests, SQL, source compiled in |
| Loaded at run time by the server | Validated at startup; firmware and UI parts generated and compiled |
| Third-party, arbitrary | First-party, in this one audited repository, released by tag |
| Could touch the raw path | Cannot — see below |
| Invisible in the store digest | Folded into store identity and reproducibility |

**A module never** sees plaintext bundle bytes, holds or derives a key, influences whether
a receipt verifies, influences a prune, adds a network listener, or writes to the store.
That is **structural**: the manifest schema sets `additionalProperties: false` throughout,
so there is no field through which any of it could be asked for.

**No new interpreter is added on any host.** A derived value is a SQL expression evaluated
by DuckDB, which the server already runs. The only expression VM anywhere is the one the
firmware already has for `engine/v1` formulas.

And a module's reach into the car is one-way: it **states** the capture fields it needs,
and the firmware's `enginegen` resolves them against the engine profile and fails the
build when one is missing. A module can never make a dongle poll a PID. With one dongle
and no spare, that asymmetry is the whole safety argument — the failure mode is a build
error on a workstation, never a changed polling loop in a car.

## Layout

```text
modules/<id>/
├── module.yaml      the manifest: identity, requires, derives, metrics, views, queries, ui, ios
├── README.md        what it claims and where the numbers came from
├── store/           views.sql, derive notes, tests on synthetic rows
├── api/             queries.yaml — named, parameterised queries
├── ui/              a Nuxt layer: pages, components, nav entry
├── ios/             optional SwiftUI; metric descriptors are generated
├── engine/          the capture fields it needs, for enginegen
└── analysis/        labels, ranges, warn and check limits per engine
```

Only `module.yaml` and `README.md` are required. A module may legitimately be **nothing
but a manifest**, and the one module here is exactly that.

## modgen

The sibling of the firmware's `enginegen`: same language, same shape, same rule that it
enforces what a JSON Schema cannot.

```sh
make validate      # every manifest, against the pinned contracts
make list          # what is here
make identity      # the module-set identity string
make vectors       # contracts/module/v1's own vectors, against this implementation
make check         # all of the above, plus cargo test — what CI runs

tools/modgen/target/release/modgen gen --target fields --out build/fields.txt
tools/modgen/target/release/modgen gen --target go     --out build/modules_gen.go
```

`modgen` is the **second independent implementation** of `module/v1`, which is item two
on that contract's release gate. The first is `tools/modulecheck` in the contracts
repository, in Go. This one was written from the spec rather than ported from it — two
readers written from one head agree on that head's mistakes.

There is deliberately no `--update`: a checker that can rewrite the vectors it is checked
against is not a check.

### What it checks that a schema cannot

- `id` equals the module's directory name.
- Exactly one module owns each derived column, and derivation order has no cycle — while
  a legitimate *chain* is still allowed, because `fuel-economy` needs the lambda
  `fuel-mixture` derives.
- Every `requires.store` entry exists in `store/v1`, or is a column some module derives.
- Every `requires.engine_fields` entry is a real `engine/v1` capture field.
- **The capture-field vocabulary has not drifted** between `engine/v1` and `module/v1`.
  `engine/v1` exists *because* two copies of one vocabulary drifted apart; this is the
  guard against repeating it.
- A metric reads a view its own module declares, or a grandfathered `store/v1` one.
- A query's parameters match its SQL **in both directions**, so a query can neither read
  an unbound parameter nor silently ignore an argument a caller passed.
- A query is one `SELECT` with no semicolon outside a string literal.
- A stub claims nothing.

### Targets

| Target | For | Status |
|---|---|---|
| `fields` | the firmware's `enginegen`: the union of required capture fields | **works** |
| `go` | the server: the module set, its identity, derivations, views and queries, as data | **works** |
| `nuxt`, `swift` | the web layer and the app | **absent on purpose.** They land with M4 and M6, when there is a consumer to shape them. Generating artefacts nobody reads would be guessing, and the guess would be committed |

## CI

One self-hosted runner, registered to this repository and installed by
[`tools/runner/install-runner.sh`](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/tools/runner/install-runner.sh)
in the front-door repository, as a rootless Podman container under a shared memory cap.

`tests/check-runners.sh` enforces what GitHub has no setting for: every job on the
self-hosted runner and never a hosted one, every job carrying the same-repository guard
so a fork's pull request cannot reach it, and no `pull_request_target` or `workflow_run`.
`tests/check-runners-selftest.sh` proves that check can actually fail.

## Contracts

This repository pins the protocols it was built against in
[`contracts.lock`](contracts.lock) — the tag **and** the commit — and fetches them into
`.contracts/`:

```sh
scripts/fetch-contracts.sh
```

`CAIRN_CONTRACTS=<dir>` overrides the fetch, so a contract and a module can change
together on a laptop. Current pin: **`contracts-v0.4.0`**, for
[`module/v1`](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/tree/main/contracts/module/v1),
[`engine/v1`](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/tree/main/contracts/engine/v1)
and
[`store/v1`](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/tree/main/contracts/store/v1).

Adding a capture field means a change to **two** files in the contracts repository —
`engine/v1`'s `$defs/field` and `module/v1`'s `$defs/engine_field` — a new contracts tag,
and a bump here. `modgen` refuses to run when those two have drifted.

## What is not here yet

Honest list, so nothing reads as further along than it is:

- **One module of six exists.** `boost` is a manifest and a README; `fuel-mixture`,
  `fuel-economy`, `driving-style`, `speedometer-check` and `places` are named in
  [MODULES.md](MODULES.md) and not written. `fuel-mixture` is deliberately first of the
  five, because it exercises the generic machinery hardest.
- **No module view, metric or named query exists anywhere.** `boost` claims a derivation
  and the four capture fields it needs, and nothing else — a metric must name a view this
  module creates, and the views have not moved out of the server yet. Claiming them in a
  manifest before they exist would be a manifest that lies.
- **No `ui/`, `ios/` or `analysis/` directories**, for the same reason.
- **Only the server consumes `modgen`'s output.** The web, app and firmware halves are
  M4–M6 of the plan, which is why the `nuxt` and `swift` emit targets are absent.
- **`module/v1` is draft** and so is this.

What *is* wired, so the list is not read as "nothing works": the server's
`internal/modules` loads and validates a module set at runtime, resolves its requirements
against the live catalogue and orders derivations topologically; `applyDerivations` runs
them between the row load and the views; and `boost.boost_psi` is a real module-owned
column. A module set that will not load is **fatal** there, because a module owns a
column's definition.

## Identity runs ahead of the pin

`module/v1` §7 used to hash a module's **whole directory**, which meant editing a
`README.md` changed the module's digest, the module-set identity and therefore — once the
server folds it in — the store contract digest. A prose fix would have made a rebuilt
store look like a different store. That is a false mismatch, and the project's fifth
invariant is worth less every time it cries wolf.

The contract was corrected: a module hashes over **its manifest and every file the manifest
names** (`views[]`, `queries`, and any later key whose value is a path). No
"except documentation" carve-out, and it extends by itself. `modgen` implements the
corrected rule, and `the_hash_covers_declarations_and_not_prose` holds it to both halves —
prose and undeclared files must not move the hash; the manifest and a declared view must.

**So `modgen` is ahead of [`contracts.lock`](contracts.lock) on this one point**, and the
tag it needs now exists. `contracts-v0.4.0` carries the old §7 wording; the correction
landed afterwards, and the server's `internal/modules` implements the same corrected rule.
Nothing is broken by the gap — identity is not yet folded into the store digest — but the
pin should move, and **this paragraph should go with it**.

## Licence

[Blue Oak Model License 1.0.0](LICENSE), as the rest of the project.
