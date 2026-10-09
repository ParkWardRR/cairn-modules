# boost

Gauge pressure and what the engine did with it: boost against RPM, wide-open-throttle
pulls, and peak boost per trip.

**Status: `derived`.** It claims one derivation and the capture fields that derivation
needs. It is still partial: no metric, no view, no page, no app gauge — because a metric
must name a view this module creates, and the views have not moved yet. Claiming them
before they exist would be a manifest that lies.

## What it claims now

`boost.boost_psi` — gauge pressure in psi, **grandfathered**. `store/v1` declares the
column and the server's decode path computes it today; this takes over the *definition*,
not the values.

```sql
CASE WHEN map_kpa IS NOT NULL AND baro_kpa IS NOT NULL
  THEN (map_kpa::DOUBLE - baro_kpa::DOUBLE) * 0.1450377 END
```

Held to `format.OBDExtended.BoostPSI()` by the server's
`TestBoostPSIDerivationReproducesTheReference`, over every manifold value 0..255 against
eight barometric cases including 0, plus both absent cases — 2,056 combinations.

**Two things the expression deliberately does not do**, both of which an earlier draft got
wrong:

- It does **not** exclude a saturated manifold reading (`map_kpa < 255`). Saturation is
  excluded by the *views* that read this column, never by the column. Excluding it here
  would silently turn every saturated sample's boost into `NULL`.
- It does **not** treat a barometric reading of 0 as absence (`baro_kpa > 0`). The
  reference treats 0 as a reading.

That draft was in the contract's own `valid-boost.json` vector, and it took evaluating the
expression against the reference to notice. The server's
`TestTheRejectedExpressionReallyDiffers` now pins it so it cannot come back believed
equivalent.

## What it will claim

| Manifest key | Value |
|---|---|
| `requires.store` | `boost.map_kpa`, `boost.baro_kpa`, `obd.rpm`, `obd.throttle_pct` |
| `requires.engine_fields` | `map_kpa`, `baro_kpa`, `rpm`, `throttle_pct` |
| `derives` | `boost.boost_psi` — gauge pressure, `(map_kpa - baro_kpa) * 0.1450377` |
| `metrics` | `boost_psi`, one observation per WOT pull, from `v_pulls.peak_boost_psi` |
| `views` | the module's own form of `v_boost_curve` and `v_pulls` |
| `ui` | `/boost`, a single-vehicle page |

## Two things the migration must get right

**`boost.boost_psi` is grandfathered.** `store/v1` declares the column and the server
computes it in its decode path today. This module may take over its *definition*, but the
column, its type and its values must not change — proven against the committed
`format/v3` conformance vectors, not by inspection. The reference implementation in the
server's `format` package stays as the thing those vectors pin; the module's SQL is tested
against it, not instead of it.

**Null, never zero, when MAP is saturated.** A saturated reading (≥ 255 kPa) is not a
measurement of 0 psi, and a barometric reading of 0 means nobody told us the altitude.
Both must produce `NULL`, because a plotted zero is a claim and a gap is not.

## Where its logic lives today

| Surface | Path |
|---|---|
| Server | `v_boost_curve`, `v_pulls` in `internal/tsdb/schema.go`; `OBDExtended.BoostGaugeKPa` / `BoostPSI` in `format/payload.go`; `insight.Boost` |
| Web | `app/pages/boost.vue`; `server/api/analytics/boost-curve`, `boost-detail`, `pulls`; peak boost in `trips/[bootId]/insights` and `dashboard/highlights` |
| iOS | `boostKpa` / `hasBoost` in `PayloadDecoder.swift`; the gauge at `MainView.swift:325` |
| Dongle | PIDs `0x0B` (MAP) and `0x33` (barometric) in the N20 engine profile |

## Sources

- MAP minus barometric is gauge pressure: `contracts/format/v3/spec.md` §4.10, and the
  server's `OBDExtended.BoostGaugeKPa` as the reference implementation.
- The WOT pull definition — throttle ≥ 70%, RPM rise > 500, at least two samples,
  MAP-saturated rows excluded — is `store/v1`'s `v_pulls`.
- The B58 in the car has **no documented boost limits**, so the analysis half will be a
  stub for it: readings shown, no verdict. See `engine/v1` §4 on why `null` means "not
  known" and never "no limit".

## A caveat worth carrying forward

Throttle position comes from PID `0x11`, the throttle plate angle, which on this
drive-by-wire engine does not reach 100% at wide-open throttle — it peaked at 77% across
the 2026-10-07 drive and read 32–34% during the one confirmed boost event. **So the
capture cannot currently say whether a pull was WOT**, and this module's pull detection
inherits that. The accelerator-pedal PIDs (`0x49`, `0x4A`) are probed but unproven on this
ECU; if one answers, the pull definition should move to it.
