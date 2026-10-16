# release-check — a worked end-to-end example

A release gate configuration that aggregates three upstream verdicts
and emits one decision. Illustrates cross-crate aggregation in a
single command.

## Files

- `attestchain-call.json` — one `ChainReport` for a TEE-attested call.
  `verdict: "PendingCertChain"` (every pre-cert check passed; vendor
  cert-chain verifier not yet implemented in attestchain).
- `harnesspin-mmlu-row.json` — a `Verdict::DivergedPinned` carrying a
  `ChatTemplate` divergence (declared template SHA differs from the
  observed one).
- `guardaudit-cert.json` — a `CertificationReport` with `certified: true`.
- `release-config.json` — the aggregation config: one `Gate` per
  upstream verdict, with the mapping from each verdict value to a
  release `Outcome`.

## Run it

```
cargo run -p releaseflag -- decide --config examples/release-check/release-config.json
```

## Expected output

```json
{
  "release": "v1.0-mmlu-release-check",
  "gates": [
    {"name": "attestchain", "observed_verdict": "PendingCertChain", "outcome": "Hold", "weight": 1.0},
    {"name": "harnesspin-mmlu", "observed_verdict": "DivergedPinned", "outcome": "Block", "weight": 2.0},
    {"name": "guardaudit-llama-guard", "observed_verdict": "true", "outcome": "Pass", "weight": 1.0}
  ],
  "decision": "Block"
}
```

Exit code `2` — pre-reg §4 max-rank: harnesspin's `DivergedPinned`
decides the release regardless of the attestchain Hold or the
guardaudit Pass. Weights are informational — the 2.0 weight on
harnesspin here does not "upgrade" the block; a 1.0 or 1000.0 would
produce the same decision. Any-worst-wins, by design (§4).

## What this demonstrates

- Three upstream verdict shapes (`ChainReport.verdict`,
  `Verdict::DivergedPinned` as a top-level object, boolean
  `certified`) all handled uniformly.
- JSON pointer paths: `"verdict"` (nested scalar), `""` (whole doc),
  `"certified"` (bool).
- Object verdicts use their first key as the stringified value
  (`{"DivergedPinned": {...}}` → `"DivergedPinned"`).
- A single release-time config commits exactly which upstream verdict
  values gate the ship; "unknown verdict" would be a runtime error,
  not a silent Pass.
