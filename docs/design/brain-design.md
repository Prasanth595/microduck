# MicroDuck 2.0 local behaviour brain

The existing neural policies are motor skills, not a general reasoning system. The 2.0 brain is a
separate, local deliberation layer that chooses goals and expressions from explicit evidence; it
may request an installed named skill but never writes joints, changes safety limits, or supersedes
a person holding the controller.

Initial expressions are `resting`, `curious`, `warm`, `recovering`, and `yielding`. The first
deployable body-language skills are episodic ONNX policies: `polite-bow`, `curious-tilt`,
`happy-bob`, and `settle`. Each must retain the 61D observation contract, be trained and exported
through `microduck_rl`, pass simulation rehearsal, and be added with `robotctl policy add`.

The brain must remain disabled from autonomous actuation until a perception adapter supplies
consented, local evidence and the authority gate confirms no human is controlling the robot.

## Mobility learning from mistakes

`robotd` now keeps a separate, persistent mobility memory at
`/var/lib/robot/mobility-memory.json` (override it with `--brain-memory`). A detected fall is
recorded immediately as an **unknown** incident with a non-blocking lock; saving happens once a
minute and on clean shutdown. An unreadable or schema-incompatible memory makes `robotd` refuse
to start rather than erase or replace the duck's prior evidence.

A future local perception adapter can label an incident or a verified safe passage as one of:
`low_obstacle`, `uneven_surface`, or `slippery_surface`. The learning rule is deliberately small
and auditable:

- one incident changes nothing;
- two matching incidents recommend `slow_and_scan`;
- three incidents plus two verified safe passages may only **consider** a separately reviewed
  step-over policy.

The learner never modifies policy weights, gait gains, safety limits, or joint targets. In
particular, it cannot turn an unlabelled fall into a step-over attempt. That last step requires a
validated locomotion policy, simulation rehearsal, a current perception frame, and the normal
human-authority gate.

### Companion progress readout

The memory JSON is a local-first, read-only companion data contract. Its
`learning_readiness_percent` is derived from cue coverage (45 points), repeated observations
(35), and safe-passage reliability (20). It is a **mobility-evidence readiness** indicator, not
an intelligence, consciousness, or safety score. A mobile companion should display it as
“Mobility learning readiness” with the raw per-cue incidents and safe passages next to it; this
prevents a reassuring percentage from hiding thin or poor-quality evidence. No network listener
is opened by this feature, so a future phone app must use the existing authenticated/LAN robot
transport rather than exposing the memory file directly.

## Voice vocabulary

The voice bank is an expressive channel alongside body language, not a human-language model.
MicroDuck 2.0 adds `whistle` (a clean high contact call) and `warble` (a short three-note
conversational trill), plus `signal` (an original electronic companion call), to the existing
`robot.sound` tags. All are synthesized locally with the
same per-duck personality and randomized variants as every existing call. The behavior brain may
eventually select an affect-to-voice suggestion, but it cannot speak arbitrary text, transmit
audio, change volume limits, or bypass the normal `robot.sound` authority path.
