# releaseflag pre-registration

Locked before the first release-time aggregation ships. Append-only.

## 1. Scope

releaseflag is the one layer above the pre-registered bench crates. It
answers one question at release time: given the committed verdict
JSONs from any subset of {attestchain, contamcheck, jailbreakreplay,
toolguard, guardaudit, agentreplay, budgetguard, shadowtrack,
kernelwitness, provenancepack, harnesspin}, is the release PASS,
HOLD, or BLOCK? It does not read model outputs, does not compute
metrics, and does not override an upstream verdict. The upstream
crates each hold the detail; releaseflag holds the ship-gate.

## 2. Inputs

A `ReleaseConfig` names one `Gate` per upstream verdict. A gate
carries: the result file path, a `/`-separated pointer to the scalar
verdict field, an exhaustive mapping from verdict value to
`Outcome { Pass | Hold | Block }`, and an informational weight.
Relative paths resolve against the config file's directory.

## 3. Outcome mapping

Every upstream verdict value a gate might observe must have an entry
in `Gate.map`. An unmapped value is a release-blocking error, not a
silent default to Pass — see §5 (no-silent-defaults). The author of
the release config is responsible for enumerating the known values of
each upstream verdict and classifying them; releaseflag refuses to
guess.

Object-shaped verdicts (e.g. jailbreakreplay's
`ReplicationReportVerdict::Scored(ReplicationVerdict)` which
serialises as `{"Scored": "Reproduced"}`) use their first key as the
tag. The release config maps the tag, not the inner value; if the
distinction between `Scored(Reproduced)` and `Scored(Diverged)`
matters at release time, author two gates on the same file with
deeper verdict paths.

## 4. Decision rule

`Decision ∈ {Pass, Hold, Block}` is the max over `Outcome` by rank:
Block > Hold > Pass. Equivalently:

- BLOCK if any gate's outcome is Block.
- HOLD if no gate is Block and at least one is Hold.
- PASS if every gate is Pass.

Weights are informational — they appear on each `GateResult` for
downstream slicing but do not enter the decision. "The worst gate
decides" matches how real release reviews work and removes an
incentive to tune weights until the number flips.

## 5. No silent defaults

Three sources of silence are each explicit errors:

- **Unmapped value** — a verdict value with no entry in `map` errors
  with the gate and value named; the author decides what it means,
  releaseflag does not.
- **Missing verdict field** — a result file whose configured
  `verdict_path` resolves to no value errors with the gate and path
  named; a Pass that depends on the field being absent is a bug.
- **No gates** — an empty `gates` list errors with `NoGates`; a
  release with nothing to check is not a Pass, it is a configuration
  error.

## 6. Exit codes

The CLI exits 0 on Pass, 1 on Hold, 2 on Block, 3 on error. A CI job
reads the exit code; the JSON report is for the human review.

## 7. What this crate does NOT do

- Compute metrics. Each upstream crate produces its own verdict; this
  layer only aggregates.
- Replace upstream pre-reg. Each gate's mapping is a release-level
  decision ("is PendingCertChain acceptable for THIS release?"), not
  a change to attestchain's pre-reg.
- Reorder, filter or re-weight by politics. The decision rule is
  deterministic and max-over-outcome; a release that wants to ship
  despite a BLOCK must remove that gate from the config, which is a
  visible change in review.

## 8. Openly tracked gaps

- No live upstream drivers. releaseflag reads result files; it does
  not drive the upstream crates to produce them. The intended flow is
  `attestchain audit ...` → `releaseflag decide ...`, with CI
  stitching them.
- No "red-team override" for Hold → Pass. If a release team wants to
  accept a Hold, they re-run with the gate removed, which is a
  visible config change.
- No bundled release config template. A future `releaseflag init`
  that stubs out one gate per installed bench crate would reduce the
  cost of adopting the crate.
