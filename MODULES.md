# The catalogue

One row per module, with what it actually claims today rather than what it is meant to.
`modgen list` prints the same thing from the manifests.

| Module | Status | Claims today | Will carry |
|---|---|---|---|
| [`boost`](modules/boost/) | `derived` | One derivation — `boost.boost_psi`, grandfathered — and the four capture fields it needs. No metric, view or page yet: a metric must name a view this module creates, and the views have not moved | Boost against RPM, WOT pull detection, peak boost, the `/boost` page |

## Planned, not yet present

From [the module system plan](https://github.com/ParkWardRR/cairn-driving-log-selfhosted/blob/main/docs/module-system-plan.md)
§8. Each names where its logic lives today, so the migration is a move and not a rewrite.

| Module | Lives today in |
|---|---|
| `fuel-mixture` | `fuel.vue`, `analytics/trim-map`, `analytics/fuel-health`, `v_trim_map`, the server's `internal/insight`, and the metric list inside `v_metric_samples` / `v_health_stats` / `v_tune_effect` |
| `fuel-economy` | `economy.vue`, `analytics/fuel-economy`, `useFuelMath.ts`, `trips/[bootId]/fuel`, the ethanol blend in `stores/ui.ts` |
| `driving-style` | `behavior.vue`, `analytics/imu` |
| `speedometer-check` | `calibration.vue`, `analytics/speed-agreement`, `v_speed_agreement` |
| `places` | `places.vue`, `places.get`, `places/saved/*`, `shared/utils/placeKinds.ts` |

`fuel-mixture` is deliberately first of the five: it exercises the generic machinery
hardest. `places` is last — it is the largest page and the least module-shaped, and it is
allowed to remain a module that contributes no metric and no capture field.

## Status words

The same three as `engine/v1`, with the same meanings:

- **`stub`** — identity only. Claims nothing, and a validator rejects a stub that states
  anything. Useful and honest: it reserves the id and records the intent.
- **`derived`** — extracted from code that already worked.
- **`verified`** — checked against the real car.
