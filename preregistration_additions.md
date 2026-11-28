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

## 2026-11-28T00:47:25Z window-sim proposal:69f0ead2e8c740ebf04aeae509b4ee95755937a2cb1ca63c54da3f5dd3f853bd

Hypothesis: on 1000 seeded SIMULATED release runs of a 3-gate config where
every gate is truly healthy and each reading is independently a spurious
Block with probability 0.05, a single-reading aggregate (max-rank rule)
flips to Block in a share of runs near 1-(0.95)^3 = 0.1426, while a
5-reading per-gate majority window (3 of 5 bad readings count a gate as
Block) flips in under one tenth of that share. The decision rule is
unchanged; only the reading policy differs.

Metric: /window/flip_rate_k1.
Expected: near 0.1426, range 0.11 to 0.18; flip_rate_k5 below one tenth of
flip_rate_k1.
Falsifier: flip_rate_k1 outside 0.11 to 0.18, or flip_rate_k5 at least one
tenth of flip_rate_k1.
