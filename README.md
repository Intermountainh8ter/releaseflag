# releaseflag

**Release-time aggregator across the pre-registered bench suite.**
Reads committed verdict JSONs from any subset of the eleven upstream
crates and emits one `PASS | HOLD | BLOCK` decision. No metrics,
no model inference, no override of an upstream verdict — this is the
one layer above them that turns N verdicts into a ship-gate.

![status: initial scaffold](https://img.shields.io/badge/status-initial%20scaffold-lightgrey)
![language: Rust](https://img.shields.io/badge/language-Rust-blue)
![license: MIT](https://img.shields.io/badge/license-MIT-blue)

## Why

The ten pre-reg bench crates each decide one question: did the
calibration pass, did the chain of custody hold, did the model reproduce
the paper's attack-success rate. Each publishes its verdict. At release
time, a reviewer still has to open every verdict file and run the
logic in their head. releaseflag is that logic as a committed Rust
decision rule, so CI reads an exit code and the review opens one
report.

## What is in this scaffold

- `ReleaseConfig { gates: Vec<Gate> }` — one `Gate` per upstream
  verdict: `name`, `file`, `verdict_path` (a `/`-separated pointer),
  exhaustive `map` from verdict value to `Outcome`, informational
  `weight`.
- `aggregate(&config, &base_dir) -> Report` — reads each gate's file,
  extracts the verdict, maps to Outcome, folds into the final
  `Decision`.
- Decision rule (pre-reg §4): max over outcome by rank Block > Hold
  > Pass. Weights are informational.
- No silent defaults (pre-reg §5): unmapped value, missing verdict
  field, and empty `gates` list each error with the gate named.
- CLI: `releaseflag decide --config release.json` exits 0 / 1 / 2
  for Pass / Hold / Block, or 3 on configuration error.

## Example

```json
{
  "name": "v1.3-ship-check",
  "gates": [
    {
      "name": "attestchain",
      "file": "attestchain/results/call_01.json",
      "verdict_path": "verdict",
      "map": {
        "Attested": "Pass",
        "PendingCertChain": "Hold",
        "Broken": "Block"
      },
      "weight": 1.0
    },
    {
      "name": "harnesspin",
      "file": "harnesspin/results/mmlu.json",
      "verdict_path": "verdict",
      "map": {
        "Matched": "Pass",
        "DivergedPinned": "Block",
        "Unverified": "Hold"
      },
      "weight": 2.0
    }
  ]
}
```

A reviewer looking at the output sees exactly which gate decided and
why; CI reads the exit code.

## What this crate does NOT do

- Compute metrics. Each upstream crate owns its own numbers.
- Replace an upstream pre-reg. Each gate's mapping is a release-level
  classification, not a change to the upstream.
- Hide a BLOCK. Shipping despite a Block requires removing the gate
  from the config — a visible change in review.

## License

MIT.
