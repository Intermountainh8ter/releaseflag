//! Release-time aggregator across the pre-registered bench suite.
//!
//! Each upstream crate (attestchain, contamcheck, jailbreakreplay,
//! toolguard, guardaudit, agentreplay, budgetguard, shadowtrack,
//! kernelwitness, provenancepack, harnesspin) emits its own verdict
//! shape — `ChainReport::verdict`, `Verdict`, `Certification::certified`,
//! `ReplicationReportVerdict`, and so on. releaseflag is the one layer
//! above them: given a `ReleaseConfig` that lists one `Gate` per
//! upstream verdict, it reads each JSON, maps the verdict value to a
//! `Outcome { Pass | Hold | Block }`, and aggregates into a single
//! `Decision`.
//!
//! Aggregation rule (pre-reg §4): the release is BLOCK if any gate is
//! Block; HOLD if any gate is Hold and none is Block; PASS otherwise.
//! Weights are informational — they appear on the result for
//! downstream slicing, but the decision rule is any-worst-wins. "The
//! worst gate decides" matches how real release reviews work and
//! removes an incentive to tune the weights until the number flips.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Outcome {
    Pass,
    Hold,
    Block,
}

impl Outcome {
    fn rank(&self) -> u8 {
        match self {
            Outcome::Pass => 0,
            Outcome::Hold => 1,
            Outcome::Block => 2,
        }
    }
}

/// Pre-reg §4 final decision.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Hold,
    Block,
}

impl From<Outcome> for Decision {
    fn from(o: Outcome) -> Self {
        match o {
            Outcome::Pass => Decision::Pass,
            Outcome::Hold => Decision::Hold,
            Outcome::Block => Decision::Block,
        }
    }
}

/// One upstream gate: which file, which JSON path has the verdict
/// value, how each possible value maps to an `Outcome`, and an
/// informational weight.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Gate {
    pub name: String,
    /// Path to the upstream result JSON. Relative paths are resolved
    /// against the config file's directory.
    pub file: PathBuf,
    /// JSON pointer-style path to the scalar verdict value in the file
    /// (e.g. `"verdict"` or `"verdict/Scored/0"`). Dot-free: segments
    /// separated by `/`.
    pub verdict_path: String,
    /// Exhaustive mapping from the stringified verdict value to an
    /// Outcome. "Exhaustive" because an unknown value never silently
    /// defaults to Pass; releaseflag refuses to aggregate when a value
    /// has no mapping.
    pub map: BTreeMap<String, Outcome>,
    /// Informational weight (e.g. 1.0, 2.0). Not used in the decision.
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReleaseConfig {
    pub name: String,
    pub gates: Vec<Gate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateResult {
    pub name: String,
    pub observed_verdict: String,
    pub outcome: Outcome,
    pub weight: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Report {
    pub release: String,
    pub gates: Vec<GateResult>,
    pub decision: Decision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AggregateError {
    IoError { gate: String, msg: String },
    JsonError { gate: String, msg: String },
    MissingVerdict { gate: String, path: String },
    UnmappedValue { gate: String, value: String },
    NoGates,
}

impl std::fmt::Display for AggregateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError { gate, msg } => write!(f, "gate `{gate}`: cannot read file: {msg}"),
            Self::JsonError { gate, msg } => write!(f, "gate `{gate}`: malformed JSON: {msg}"),
            Self::MissingVerdict { gate, path } => {
                write!(f, "gate `{gate}`: no value at JSON path `{path}`")
            }
            Self::UnmappedValue { gate, value } => write!(
                f,
                "gate `{gate}`: observed verdict `{value}` has no entry in `map` — refuse to default to Pass"
            ),
            Self::NoGates => f.write_str("release config has no gates"),
        }
    }
}

impl std::error::Error for AggregateError {}

fn pointer_from_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        let mut s = String::with_capacity(trimmed.len() + 1);
        for seg in trimmed.split('/') {
            s.push('/');
            s.push_str(seg);
        }
        s
    }
}

fn stringify_verdict(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "null".into(),
        // For object/array verdicts (e.g. `ReplicationReportVerdict::Scored(..)`),
        // use the first key of an object (variant tag) or the JSON itself.
        serde_json::Value::Object(obj) => obj
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "{}".to_string()),
        other => other.to_string(),
    }
}

/// Aggregate across every gate in the config. `base_dir` is the folder
/// relative paths in `Gate::file` resolve against; pass the directory
/// the config file lives in.
pub fn aggregate(config: &ReleaseConfig, base_dir: &Path) -> Result<Report, AggregateError> {
    if config.gates.is_empty() {
        return Err(AggregateError::NoGates);
    }
    let mut results: Vec<GateResult> = Vec::with_capacity(config.gates.len());
    for g in &config.gates {
        let path = if g.file.is_absolute() {
            g.file.clone()
        } else {
            base_dir.join(&g.file)
        };
        let text = std::fs::read_to_string(&path).map_err(|e| AggregateError::IoError {
            gate: g.name.clone(),
            msg: e.to_string(),
        })?;
        let doc: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| AggregateError::JsonError {
                gate: g.name.clone(),
                msg: e.to_string(),
            })?;
        let p = pointer_from_path(&g.verdict_path);
        let v = doc
            .pointer(&p)
            .ok_or_else(|| AggregateError::MissingVerdict {
                gate: g.name.clone(),
                path: g.verdict_path.clone(),
            })?;
        let as_str = stringify_verdict(v);
        let outcome = g
            .map
            .get(&as_str)
            .copied()
            .ok_or_else(|| AggregateError::UnmappedValue {
                gate: g.name.clone(),
                value: as_str.clone(),
            })?;
        results.push(GateResult {
            name: g.name.clone(),
            observed_verdict: as_str,
            outcome,
            weight: g.weight,
        });
    }
    let decision = results
        .iter()
        .map(|r| r.outcome)
        .max_by_key(|o| o.rank())
        .map(Decision::from)
        .expect("at least one gate (NoGates would have returned above)");
    Ok(Report {
        release: config.name.clone(),
        gates: results,
        decision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!(
            "releaseflag-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn write_file(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    fn verdict_map(pairs: &[(&str, Outcome)]) -> BTreeMap<String, Outcome> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect()
    }

    #[test]
    fn all_pass_yields_pass() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": "Attested"}"#);
        write_file(&dir, "b.json", r#"{"verdict": "Matched"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![
                Gate {
                    name: "attestchain".into(),
                    file: "a.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[
                        ("Attested", Outcome::Pass),
                        ("PendingCertChain", Outcome::Hold),
                        ("Broken", Outcome::Block),
                    ]),
                    weight: 1.0,
                },
                Gate {
                    name: "harnesspin".into(),
                    file: "b.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[
                        ("Matched", Outcome::Pass),
                        ("DivergedPinned", Outcome::Block),
                        ("Unverified", Outcome::Hold),
                    ]),
                    weight: 2.0,
                },
            ],
        };
        let r = aggregate(&config, &dir).expect("ok");
        assert_eq!(r.decision, Decision::Pass);
        assert_eq!(r.gates.len(), 2);
    }

    #[test]
    fn any_block_yields_block() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": "Attested"}"#);
        write_file(&dir, "b.json", r#"{"verdict": "DivergedPinned"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![
                Gate {
                    name: "attestchain".into(),
                    file: "a.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[("Attested", Outcome::Pass)]),
                    weight: 1.0,
                },
                Gate {
                    name: "harnesspin".into(),
                    file: "b.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[
                        ("Matched", Outcome::Pass),
                        ("DivergedPinned", Outcome::Block),
                    ]),
                    weight: 1.0,
                },
            ],
        };
        let r = aggregate(&config, &dir).expect("ok");
        assert_eq!(r.decision, Decision::Block);
    }

    #[test]
    fn hold_without_block_yields_hold() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": "PendingCertChain"}"#);
        write_file(&dir, "b.json", r#"{"verdict": "Matched"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![
                Gate {
                    name: "attestchain".into(),
                    file: "a.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[
                        ("Attested", Outcome::Pass),
                        ("PendingCertChain", Outcome::Hold),
                    ]),
                    weight: 1.0,
                },
                Gate {
                    name: "harnesspin".into(),
                    file: "b.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[("Matched", Outcome::Pass)]),
                    weight: 1.0,
                },
            ],
        };
        let r = aggregate(&config, &dir).expect("ok");
        assert_eq!(r.decision, Decision::Hold);
    }

    #[test]
    fn unmapped_value_errors_not_silent_pass() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": "SomethingNew"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![Gate {
                name: "attestchain".into(),
                file: "a.json".into(),
                verdict_path: "verdict".into(),
                map: verdict_map(&[("Attested", Outcome::Pass)]),
                weight: 1.0,
            }],
        };
        match aggregate(&config, &dir) {
            Err(AggregateError::UnmappedValue { gate, value }) => {
                assert_eq!(gate, "attestchain");
                assert_eq!(value, "SomethingNew");
            }
            o => panic!("wrong variant: {o:?}"),
        }
    }

    #[test]
    fn missing_verdict_names_gate_and_path() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"other_field": "x"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![Gate {
                name: "attestchain".into(),
                file: "a.json".into(),
                verdict_path: "verdict".into(),
                map: verdict_map(&[("x", Outcome::Pass)]),
                weight: 1.0,
            }],
        };
        match aggregate(&config, &dir) {
            Err(AggregateError::MissingVerdict { gate, path }) => {
                assert_eq!(gate, "attestchain");
                assert_eq!(path, "verdict");
            }
            o => panic!("wrong variant: {o:?}"),
        }
    }

    #[test]
    fn object_verdict_uses_first_key_as_tag() {
        // jailbreakreplay's Scored(Reproduced) renders as {"Scored": "Reproduced"}
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": {"Scored": "Reproduced"}}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![Gate {
                name: "jailbreakreplay".into(),
                file: "a.json".into(),
                verdict_path: "verdict".into(),
                map: verdict_map(&[
                    ("Scored", Outcome::Pass),
                    ("GraderUncalibrated", Outcome::Block),
                    ("TemplateDrift", Outcome::Block),
                    ("TrialsBelowMinimum", Outcome::Hold),
                ]),
                weight: 1.0,
            }],
        };
        let r = aggregate(&config, &dir).expect("ok");
        assert_eq!(r.decision, Decision::Pass);
        assert_eq!(r.gates[0].observed_verdict, "Scored");
    }

    #[test]
    fn no_gates_errors() {
        let dir = tempdir();
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![],
        };
        assert_eq!(aggregate(&config, &dir), Err(AggregateError::NoGates));
    }

    #[test]
    fn absolute_file_path_resolves_without_base_dir() {
        let dir = tempdir();
        let p = write_file(&dir, "a.json", r#"{"verdict": "Attested"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![Gate {
                name: "attestchain".into(),
                file: p.clone(),
                verdict_path: "verdict".into(),
                map: verdict_map(&[("Attested", Outcome::Pass)]),
                weight: 1.0,
            }],
        };
        // Use a wrong base_dir — the absolute path should still resolve.
        let r = aggregate(&config, std::path::Path::new("/does/not/exist")).expect("ok");
        assert_eq!(r.decision, Decision::Pass);
    }

    #[test]
    fn weights_are_informational_not_decision_affecting() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": "Block"}"#);
        write_file(&dir, "b.json", r#"{"verdict": "Pass"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![
                Gate {
                    name: "small".into(),
                    file: "a.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[("Block", Outcome::Block)]),
                    weight: 0.01,
                },
                Gate {
                    name: "big".into(),
                    file: "b.json".into(),
                    verdict_path: "verdict".into(),
                    map: verdict_map(&[("Pass", Outcome::Pass)]),
                    weight: 1000.0,
                },
            ],
        };
        // Even with a 1000x-weighted Pass, a single Block decides.
        let r = aggregate(&config, &dir).expect("ok");
        assert_eq!(r.decision, Decision::Block);
    }

    #[test]
    fn deeper_json_path_navigates_nested_object() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"audit": {"outer": {"verdict": "Attested"}}}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![Gate {
                name: "attestchain".into(),
                file: "a.json".into(),
                verdict_path: "audit/outer/verdict".into(),
                map: verdict_map(&[("Attested", Outcome::Pass)]),
                weight: 1.0,
            }],
        };
        let r = aggregate(&config, &dir).expect("ok");
        assert_eq!(r.decision, Decision::Pass);
    }

    #[test]
    fn report_round_trips_json() {
        let dir = tempdir();
        write_file(&dir, "a.json", r#"{"verdict": "Attested"}"#);
        let config = ReleaseConfig {
            name: "r1".into(),
            gates: vec![Gate {
                name: "attestchain".into(),
                file: "a.json".into(),
                verdict_path: "verdict".into(),
                map: verdict_map(&[("Attested", Outcome::Pass)]),
                weight: 1.0,
            }],
        };
        let r = aggregate(&config, &dir).expect("ok");
        let s = serde_json::to_string(&r).unwrap();
        let r2: Report = serde_json::from_str(&s).unwrap();
        assert_eq!(r, r2);
    }
}
