# Pre-registration additions

## 2026-11-28T00:28:19Z missing-sweep proposal:2536d04674b939441aaf58cb594f40dd24b0dbd1936ae53117888986b92ccdca

Hypothesis: on the committed examples/release-check config (all-present
decision Block, decided by the harnesspin gate), treating a missing verdict
file as Hold changes the final decision in a strictly positive share of the
single- and pair-removal subsets (those that remove the Block-deciding
gate), whereas treating it as Block changes none. The missing-gate policy
alone, not upstream evidence, decides the outcome in those subsets.

Metric: /missing_sweep/policies/hold/decision_change_share.
Expected: 0.5 (3 of 6 subsets), range 0.4 to 0.6.
Falsifier: the Hold share equals 0, or the Block share is above 0.
The error policy counts every subset as changed (the aggregate exits 3).
