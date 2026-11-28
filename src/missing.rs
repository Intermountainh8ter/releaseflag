//! `missing-sweep`: decision under three missing-gate policies when
//! gate verdict files are removed (single gates and pairs).

use crate::{aggregate, Decision, Outcome, ReleaseConfig};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    Error,
    Hold,
    Block,
}

#[derive(Debug, Serialize)]
pub struct SubsetResult {
    pub removed: Vec<String>,
    /// `Pass|Hold|Block` or `Error` when the policy refuses to decide.
    pub decision: String,
    pub changed: bool,
}

#[derive(Debug, Serialize)]
pub struct PolicyResult {
    pub subsets: Vec<SubsetResult>,
    pub changed_count: usize,
    pub decision_change_share: f64,
}

#[derive(Debug, Serialize)]
pub struct MissingSweep {
    pub release: String,
    pub all_present_decision: Decision,
    pub n_subsets: usize,
    pub policies: BTreeMap<String, PolicyResult>,
}

fn name(d: Decision) -> String {
    format!("{d:?}")
}

/// Sweep every removal subset of size 1 and 2 (never all gates).
pub fn missing_sweep(cfg: &ReleaseConfig, base: &Path) -> Result<MissingSweep, String> {
    let n = cfg.gates.len();
    let full = aggregate(cfg, base).map_err(|e| e.to_string())?;
    let outcomes: Vec<Outcome> = full.gates.iter().map(|g| g.outcome).collect();
    let mut subsets: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        if n > 1 {
            subsets.push(vec![i]);
        }
    }
    for i in 0..n {
        for j in i + 1..n {
            if j - i > 0 && n > 2 {
                subsets.push(vec![i, j]);
            }
        }
    }
    let mut policies = BTreeMap::new();
    for (key, pol) in [("error", Policy::Error), ("hold", Policy::Hold), ("block", Policy::Block)] {
        let mut rows = Vec::new();
        for s in &subsets {
            let decision = match pol {
                Policy::Error => None,
                Policy::Hold | Policy::Block => {
                    let fill = if pol == Policy::Hold { Outcome::Hold } else { Outcome::Block };
                    let d = (0..n)
                        .map(|k| if s.contains(&k) { fill } else { outcomes[k] })
                        .max_by_key(|o| match o {
                            Outcome::Pass => 0,
                            Outcome::Hold => 1,
                            Outcome::Block => 2,
                        })
                        .map(Decision::from)
                        .expect("n >= 1");
                    Some(d)
                }
            };
            rows.push(SubsetResult {
                removed: s.iter().map(|&k| cfg.gates[k].name.clone()).collect(),
                decision: decision.map(name).unwrap_or_else(|| "Error".into()),
                changed: decision != Some(full.decision),
            });
        }
        let changed_count = rows.iter().filter(|r| r.changed).count();
        let share = if rows.is_empty() { 0.0 } else { changed_count as f64 / rows.len() as f64 };
        policies.insert(
            key.to_string(),
            PolicyResult { subsets: rows, changed_count, decision_change_share: share },
        );
    }
    Ok(MissingSweep {
        release: cfg.name.clone(),
        all_present_decision: full.decision,
        n_subsets: subsets.len(),
        policies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> (ReleaseConfig, std::path::PathBuf) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/release-check");
        let text = std::fs::read_to_string(dir.join("release-config.json")).unwrap();
        (serde_json::from_str(&text).unwrap(), dir)
    }

    #[test]
    fn example_sweep_shares() {
        let (cfg, dir) = example();
        let s = missing_sweep(&cfg, &dir).unwrap();
        assert_eq!(s.all_present_decision, Decision::Block);
        assert_eq!(s.n_subsets, 6);
        assert_eq!(s.policies["hold"].decision_change_share, 0.5);
        assert_eq!(s.policies["block"].decision_change_share, 0.0);
        assert_eq!(s.policies["error"].decision_change_share, 1.0);
    }

    #[test]
    fn single_gate_has_no_subsets() {
        let (mut cfg, dir) = example();
        cfg.gates.truncate(1);
        let s = missing_sweep(&cfg, &dir).unwrap();
        assert_eq!(s.n_subsets, 0);
        assert_eq!(s.policies["hold"].decision_change_share, 0.0);
    }
}
