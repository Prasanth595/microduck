//! Bounded local deliberation for autonomous expression.
//!
//! The neural network here selects an *affect*, never a joint target or a policy. `robotd` keeps
//! authority arbitration, safety limits and all motor writes. A human-control or fall stimulus is
//! always resolved by fixed safety rules before the network is consulted.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const INPUTS: usize = 5;
const OUTPUTS: usize = 5;

/// Persistent local mobility experience. Kept outside releases, like the vet book.
pub const DEFAULT_MOBILITY_PATH: &str = "/var/lib/robot/mobility-memory.json";

/// The cues the locomotion learner is allowed to remember.  These are deliberately coarse:
/// identifying a household object is neither reliable enough nor necessary to make a safe choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobilityCue {
    Unknown,
    LowObstacle,
    UnevenSurface,
    SlipperySurface,
}

/// A non-actuating recommendation derived from local, reviewed experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobilityAdvice {
    /// There is not enough evidence to change behavior.
    Continue,
    /// Slow down and obtain another perception frame; this never changes joint targets.
    SlowAndScan,
    /// A matching, separately validated step-over policy may be considered by the policy arbiter.
    /// It is not an instruction to execute that policy.
    ConsiderReviewedStepOver,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CueRecord {
    incidents: u32,
    safe_passages: u32,
}

/// A small, local, inspectable record of mobility outcomes.
///
/// This is intentionally not online reinforcement learning over motor commands. A fall must not
/// teach a quadruped a new gait on a living-room floor. Instead, it learns confidence in a
/// *bounded* cue-to-response rule, and only proposes a reviewed step-over skill once repeated
/// evidence warrants it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobilitySnapshot {
    schema_version: u32,
    cues: BTreeMap<MobilityCue, CueRecord>,
}

/// The read-only shape a companion can render. It intentionally contains evidence, not motor
/// commands or a way to alter the learner.
#[derive(Debug, Clone, Serialize)]
pub struct MobilityStatus {
    pub schema_version: u32,
    pub learning_readiness_percent: u8,
    cues: BTreeMap<MobilityCue, CueRecord>,
}

impl Default for MobilitySnapshot {
    fn default() -> Self {
        Self {
            schema_version: 1,
            cues: BTreeMap::new(),
        }
    }
}

impl MobilitySnapshot {
    pub fn record_incident(&mut self, cue: MobilityCue) {
        self.cues.entry(cue).or_default().incidents += 1;
    }

    pub fn record_safe_passage(&mut self, cue: MobilityCue) {
        self.cues.entry(cue).or_default().safe_passages += 1;
    }

    pub fn advice(&self, cue: MobilityCue) -> MobilityAdvice {
        let Some(record) = self.cues.get(&cue) else {
            return MobilityAdvice::Continue;
        };
        // One observation may be a bad depth frame. Two separate incidents earn a caution;
        // three, with evidence the environment can be traversed safely, can nominate a reviewed
        // step-over policy. Neither branch drives motors on its own.
        if record.incidents >= 3 && record.safe_passages >= 2 {
            MobilityAdvice::ConsiderReviewedStepOver
        } else if record.incidents >= 2 {
            MobilityAdvice::SlowAndScan
        } else {
            MobilityAdvice::Continue
        }
    }

    /// A transparent progress indicator for companion UIs, not a measure of consciousness or IQ.
    /// It rewards coverage and repeatable evidence, and it drops when incidents outweigh safe
    /// passages. It is deliberately capped below 100 until all supported cue classes have data.
    pub fn learning_readiness_percent(&self) -> u8 {
        const CUES: [MobilityCue; 3] = [
            MobilityCue::LowObstacle,
            MobilityCue::UnevenSurface,
            MobilityCue::SlipperySurface,
        ];
        let covered = CUES
            .iter()
            .filter(|cue| {
                self.cues
                    .get(cue)
                    .is_some_and(|record| record.incidents + record.safe_passages > 0)
            })
            .count() as u32;
        let evidence: u32 = self
            .cues
            .values()
            .map(|record| record.incidents + record.safe_passages)
            .sum();
        let safe: u32 = self.cues.values().map(|record| record.safe_passages).sum();
        let incidents: u32 = self.cues.values().map(|record| record.incidents).sum();
        let coverage = covered * 45 / CUES.len() as u32;
        let repetitions = evidence.min(20) * 35 / 20;
        let reliability = if safe + incidents == 0 {
            0
        } else {
            safe * 20 / (safe + incidents)
        };
        (coverage + repetitions + reliability).min(100) as u8
    }

    pub fn status(&self) -> MobilityStatus {
        MobilityStatus {
            schema_version: self.schema_version,
            learning_readiness_percent: self.learning_readiness_percent(),
            cues: self.cues.clone(),
        }
    }
}

/// Thread-safe local mobility memory. Saving happens on a timer or clean shutdown, never from
/// the control tick, so a slow eMMC cannot turn learning into a fall hazard.
#[derive(Debug)]
pub struct MobilityLearner {
    path: Option<PathBuf>,
    snapshot: Mutex<MobilitySnapshot>,
}

impl MobilityLearner {
    pub fn open(path: Option<PathBuf>) -> Result<Self, String> {
        let snapshot = match path.as_deref() {
            Some(path) if path.exists() => {
                let text = std::fs::read_to_string(path).map_err(|error| {
                    format!("cannot read mobility memory {}: {error}", path.display())
                })?;
                let snapshot: MobilitySnapshot = serde_json::from_str(&text).map_err(|error| {
                    format!("invalid mobility memory {}: {error}", path.display())
                })?;
                if snapshot.schema_version != 1 {
                    return Err(format!(
                        "unsupported mobility memory schema {}",
                        snapshot.schema_version
                    ));
                }
                snapshot
            }
            _ => MobilitySnapshot::default(),
        };
        Ok(Self {
            path,
            snapshot: Mutex::new(snapshot),
        })
    }

    pub fn record_incident(&self, cue: MobilityCue) {
        if let Ok(mut snapshot) = self.snapshot.try_lock() {
            snapshot.record_incident(cue);
        }
    }

    pub fn record_safe_passage(&self, cue: MobilityCue) {
        if let Ok(mut snapshot) = self.snapshot.try_lock() {
            snapshot.record_safe_passage(cue);
        }
    }

    pub fn snapshot(&self) -> MobilitySnapshot {
        self.snapshot
            .lock()
            .expect("mobility memory lock poisoned")
            .clone()
    }

    pub fn status(&self) -> MobilityStatus {
        self.snapshot().status()
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        let text = serde_json::to_vec_pretty(&self.status())
            .map_err(|error| format!("cannot encode mobility memory: {error}"))?;
        let parent = path
            .parent()
            .ok_or_else(|| format!("mobility memory has no parent: {}", path.display()))?;
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "cannot create mobility memory directory {}: {error}",
                parent.display()
            )
        })?;
        let temporary = path.with_extension("json.new");
        std::fs::write(&temporary, text).map_err(|error| {
            format!(
                "cannot write mobility memory {}: {error}",
                temporary.display()
            )
        })?;
        std::fs::rename(&temporary, path)
            .map_err(|error| format!("cannot install mobility memory {}: {error}", path.display()))
    }
}

impl Default for MobilityLearner {
    fn default() -> Self {
        Self::open(None).expect("in-memory mobility learner is valid")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stimulus {
    Idle,
    PersonNearby,
    GentleTouch,
    Fallen,
    HumanControl,
}

impl Stimulus {
    fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::PersonNearby => 1,
            Self::GentleTouch => 2,
            Self::Fallen => 3,
            Self::HumanControl => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affect {
    Resting,
    Curious,
    Warm,
    Recovering,
    Yielding,
}

impl Affect {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Resting,
            1 => Self::Curious,
            2 => Self::Warm,
            3 => Self::Recovering,
            4 => Self::Yielding,
            _ => unreachable!("neural output index is validated"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub affect: Affect,
    /// A requested name is advisory. The caller must still verify that this is an installed
    /// skill, that it is safe to run, and that no human has authority.
    pub requested_skill: Option<&'static str>,
}

impl Decision {
    /// Whether autonomous, low-energy expression may remain active.
    ///
    /// This is deliberately a permission, not an actuator command. The caller still owns its
    /// own fade, limits, and all of `robotd`'s authority checks.
    pub fn permits_autonomous_expression(&self) -> bool {
        !matches!(self.affect, Affect::Yielding | Affect::Recovering)
    }
}

/// A small, portable one-hidden-layer ReLU network trained by `microduck_rl/scripts/train_brain.py`.
///
/// Inputs are one-hot in this order: idle, person-nearby, gentle-touch, fallen and human-control.
/// Outputs follow [`Affect`]'s order. Its explicit shape makes inference auditable and bounded.
#[derive(Debug, Clone)]
struct NeuralBrain {
    hidden_weights: Vec<Vec<f64>>,
    hidden_bias: Vec<f64>,
    output_weights: Vec<Vec<f64>>,
    output_bias: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NeuralBrainFile {
    schema_version: u32,
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
    hidden_weights: Vec<Vec<f64>>,
    hidden_bias: Vec<f64>,
    output_weights: Vec<Vec<f64>>,
    output_bias: Vec<f64>,
}

impl NeuralBrain {
    fn from_json(json: &str) -> Result<Self, String> {
        let file: NeuralBrainFile = serde_json::from_str(json)
            .map_err(|error| format!("invalid brain model JSON: {error}"))?;
        if file.schema_version != 1 {
            return Err(format!(
                "unsupported brain model schema {}",
                file.schema_version
            ));
        }
        if file.input_size != INPUTS || file.output_size != OUTPUTS || file.hidden_size == 0 {
            return Err(format!(
                "brain model shape must be {INPUTS} inputs, non-zero hidden width, and {OUTPUTS} outputs"
            ));
        }
        if file.hidden_weights.len() != file.hidden_size
            || file.hidden_weights.iter().any(|row| row.len() != INPUTS)
            || file.hidden_bias.len() != file.hidden_size
            || file.output_weights.len() != OUTPUTS
            || file
                .output_weights
                .iter()
                .any(|row| row.len() != file.hidden_size)
            || file.output_bias.len() != OUTPUTS
        {
            return Err("brain model weight dimensions do not match its declared shape".to_owned());
        }
        if file
            .hidden_weights
            .iter()
            .flatten()
            .chain(file.hidden_bias.iter())
            .chain(file.output_weights.iter().flatten())
            .chain(file.output_bias.iter())
            .any(|weight| !weight.is_finite())
        {
            return Err("brain model weights must be finite".to_owned());
        }
        Ok(Self {
            hidden_weights: file.hidden_weights,
            hidden_bias: file.hidden_bias,
            output_weights: file.output_weights,
            output_bias: file.output_bias,
        })
    }

    fn infer(&self, stimulus: Stimulus) -> Affect {
        let mut input = [0.0; INPUTS];
        input[stimulus.index()] = 1.0;
        let hidden: Vec<f64> = self
            .hidden_weights
            .iter()
            .zip(&self.hidden_bias)
            .map(|(weights, bias)| {
                weights
                    .iter()
                    .zip(input)
                    .map(|(weight, value)| weight * value)
                    .sum::<f64>()
                    + bias
            })
            .map(|value| value.max(0.0))
            .collect();
        let (output, _) = self
            .output_weights
            .iter()
            .zip(&self.output_bias)
            .enumerate()
            .map(|(index, (weights, bias))| {
                let score = weights
                    .iter()
                    .zip(&hidden)
                    .map(|(weight, value)| weight * value)
                    .sum::<f64>()
                    + bias;
                (index, score)
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .expect("validated output width is non-zero");
        Affect::from_index(output)
    }
}

/// A local behavior selector. Without a reviewed model it uses the conservative baseline below.
#[derive(Debug, Default)]
pub struct Brain {
    neural: Option<NeuralBrain>,
}

impl Brain {
    /// Load a reviewed behavior model. Loading is separate from inference so an invalid model can
    /// be rejected before the control loop starts.
    pub fn from_model_json(json: &str) -> Result<Self, String> {
        Ok(Self {
            neural: Some(NeuralBrain::from_json(json)?),
        })
    }

    pub fn decide(&self, stimulus: Stimulus) -> Decision {
        // Safety-critical stimuli are rules, not learned behavior. A bad model cannot make a
        // fallen duck keep expressing or make human input compete with autonomy.
        if matches!(stimulus, Stimulus::Fallen | Stimulus::HumanControl) {
            return Self::baseline(stimulus);
        }
        let affect = self
            .neural
            .as_ref()
            .map(|model| model.infer(stimulus))
            .unwrap_or_else(|| Self::baseline(stimulus).affect);
        // Neural inference has no motor-policy or skill authority. A later, separately reviewed
        // skill arbiter may interpret an affect, but only after the normal `robotd` gates.
        Decision {
            affect,
            requested_skill: None,
        }
    }

    fn baseline(stimulus: Stimulus) -> Decision {
        match stimulus {
            Stimulus::Idle => Decision {
                affect: Affect::Resting,
                requested_skill: None,
            },
            Stimulus::PersonNearby => Decision {
                affect: Affect::Curious,
                requested_skill: None,
            },
            Stimulus::GentleTouch => Decision {
                affect: Affect::Warm,
                requested_skill: Some("polite-bow"),
            },
            Stimulus::Fallen => Decision {
                affect: Affect::Recovering,
                requested_skill: None,
            },
            Stimulus::HumanControl => Decision {
                affect: Affect::Yielding,
                requested_skill: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Affect, Brain, MobilityAdvice, MobilityCue, MobilitySnapshot, Stimulus};

    // The external Python trainer writes this schema; this fixture keeps the runtime contract local.
    const MODEL: &str = r#"{
        "schema_version": 1, "input_size": 5, "hidden_size": 5, "output_size": 5,
        "hidden_weights": [[1,0,0,0,0],[0,1,0,0,0],[0,0,1,0,0],[0,0,0,1,0],[0,0,0,0,1]],
        "hidden_bias": [0,0,0,0,0],
        "output_weights": [[1,0,0,0,0],[0,1,0,0,0],[0,0,1,0,0],[0,0,0,1,0],[0,0,0,0,1]],
        "output_bias": [0,0,0,0,0]
    }"#;

    #[test]
    fn trained_model_selects_an_affect_but_never_a_skill() {
        let brain = Brain::from_model_json(MODEL).unwrap();
        let decision = brain.decide(Stimulus::PersonNearby);
        assert_eq!(decision.affect, Affect::Curious);
        assert!(decision.requested_skill.is_none());
    }

    #[test]
    fn safety_rules_override_a_model() {
        let brain = Brain::from_model_json(MODEL).unwrap();
        let decision = brain.decide(Stimulus::HumanControl);
        assert_eq!(decision.affect, Affect::Yielding);
        assert!(!decision.permits_autonomous_expression());
    }

    #[test]
    fn rejects_bad_model_shapes() {
        let error = Brain::from_model_json("{\"schema_version\":1}").unwrap_err();
        assert!(error.contains("invalid brain model JSON"));
    }

    #[test]
    fn repeated_contextual_mistakes_make_a_bounded_recommendation() {
        let mut memory = MobilitySnapshot::default();
        memory.record_incident(MobilityCue::LowObstacle);
        assert_eq!(
            memory.advice(MobilityCue::LowObstacle),
            MobilityAdvice::Continue
        );
        memory.record_incident(MobilityCue::LowObstacle);
        assert_eq!(
            memory.advice(MobilityCue::LowObstacle),
            MobilityAdvice::SlowAndScan
        );
        memory.record_incident(MobilityCue::LowObstacle);
        memory.record_safe_passage(MobilityCue::LowObstacle);
        memory.record_safe_passage(MobilityCue::LowObstacle);
        assert_eq!(
            memory.advice(MobilityCue::LowObstacle),
            MobilityAdvice::ConsiderReviewedStepOver
        );
    }

    #[test]
    fn readiness_is_evidence_not_an_iq_score() {
        let mut memory = MobilitySnapshot::default();
        assert_eq!(memory.learning_readiness_percent(), 0);
        for cue in [
            MobilityCue::LowObstacle,
            MobilityCue::UnevenSurface,
            MobilityCue::SlipperySurface,
        ] {
            for _ in 0..8 {
                memory.record_safe_passage(cue);
            }
        }
        assert_eq!(memory.learning_readiness_percent(), 100);
    }
}
