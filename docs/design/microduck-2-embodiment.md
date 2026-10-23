# MicroDuck 2.0 embodiment plan

Status: proposal · Owner: MicroDuck 2.0

This is the physical plan for a larger, expressive MicroDuck 2.0. It is **not** a configuration
for the 25 cm, 800 g MicroDuck hardware, and must not be used to change torque, joint limits, or
policies on an existing robot. MicroDuck 2.0 is a new hardware family with a new simulator body,
actuator qualification, calibration record, and policy set.

## 1. Product envelope

| property | 2.0 target | decision behind it |
| --- | --- | --- |
| standing height | 38–50 cm / 15–20 in | Readable posture and head language while remaining approachable indoors. |
| lower bound | 30.5 cm / 12 in | A sub-foot build is not the large-body 2.0 target. |
| upper bound | 61 cm / 24 in | Beyond this, transport, fall energy, actuator and safeguarding requirements change materially. |
| mass | 2.5–4.0 kg provisional | Validate through CAD and fall tests; keep the centre of mass inside the foot support polygon at rest. |
| operating surface | level indoor floor first | Stairs, outdoor terrain, and unsupervised public operation are out of scope initially. |

The reference chassis targets **45 cm / 18 in**. It permits smaller 12–15 in and taller 20–24 in
editions later, but neither can reuse a learned policy or calibration merely because it looks
similar.

The accompanying [parametric CAD reference](cad/README.md) carries the same 45 cm envelope,
210 × 120 mm feet, torso clearance, electronics-tray placement and modular leg concept in
OpenSCAD. It is an early engineering communication artifact—not a fabrication release.

## 2. Mechanical structure

The body is a serviceable pelvis-and-torso frame, not a scaled print of the current duck:

```text
       expressive head
       ┌─── neck ───┐
       │ torso / compute │     removable shell, speaker, camera and status light
       └─────┬──────┘
         protected spine       battery low and central; never in the head
       ┌─────┴──────┐
   left hip       right hip    independent leg modules
      │                │
thigh → knee → ankle → foot    broad, compliant, replaceable feet
```

- **Low, central battery bay:** mechanically retained, pack-fused and removable without
  dismantling a leg. Software and the brain never receive a direct power interface.
- **Wide feet with compliant soles:** foot geometry is part of safety, state estimation and body
  language. Add toe, wheel or spring mechanisms only with sensing, a simulator model and a fall
  test plan.
- **Structural load paths:** neck, pelvis and feet use metal or validated structural composites;
  cosmetic duck shells are sacrificial covers.
- **Modular leg cartridges:** each records actuator serials, reduction ratio, limits and
  calibration revision; removing one must not recalibrate the other leg.
- **Soft external geometry:** no reachable horn, pinch gap or sharp edge around the head, neck,
  hip or ankle envelopes.

The first CAD milestone is an assembly with collision meshes, mass properties, joint hard stops,
service clearances, cable routes and a centre-of-mass envelope across every commanded pose. A
render alone is not a mechanical milestone.

## 3. Actuation, sensing and power

Scale changes load much faster than height alone suggests. Actuator selection follows torque,
thermal and fall-load calculations for the actual links; the current XL330 setup must not be
“turned up.”

| subsystem | 2.0 requirement |
| --- | --- |
| legs | New actuator/reduction selection qualified for continuous stance, recovery and bounded thermal operation. Hardware hard stops remain inside software limits. |
| torso/head | Low-inertia expression axes where useful; they fail to a non-striking pose and cannot destabilize stance. |
| sensors | Body IMU; joint position, velocity and current; battery voltage/current/temperature; and foot-contact evidence. Camera/proximity inputs are optional, never sole fall protection. |
| power | Fused pack, service disconnect, regulated compute rail, monitored motor rail and physical emergency stop that removes actuator torque without Linux. |
| wiring | Strain relief at every moving joint, protected torso routing and keyed connectors. |

Three independent safe states are required: software motion stop, commanded torque-off, and
physical power isolation. The brain may request the first only through `robotd`.

## 4. Electronics, secure radio and companion architecture

MicroDuck 2.0 uses a two-computer design: the existing Linux compute module continues to own
perception, policy inference, audio and `robotd`; a low-power **STM32WB55RG** supervisor owns
always-on power sequencing, physical safety inputs and short-range pairing. This keeps radio
parsing and battery-critical functions independent from a Linux application restart.

| block | selected component | responsibility | boundary |
| --- | --- | --- | --- |
| safety / radio supervisor | STM32WB55RG | Cortex-M4 at up to 64 MHz runs safety I/O, battery wake and the signed supervisor application; Cortex-M0+ owns the vendor wireless stack. 1 MB flash / 256 KB SRAM are budgeted between the two cores. | May cut motor enable and report state; may not command joints or alter a running policy. |
| phone link | STM32WB55 integrated BLE 5.4 | Bonded BLE pairing, low-rate status and provisioning hand-off to the Linux service. | Authenticated, rate-limited GATT bridge only; it is never a motor-control or raw-radio bridge. |
| optional sub-GHz lab module | TI CC1101 + region-specific, matched antenna | Receive-only diagnostics for a customer-owned test transmitter in a declared legal band; present only on an approved regional RF daughterboard. | No transmit firmware, remote replay, garage/barrier operation, code capture, or arbitrary frequency mode in the product. |
| proximity / owner tap | ST25R3916 + tuned 13.56 MHz loop | Read an owner-issued, signed NFC provisioning tag or service tag; report only a verified opaque credential ID to Linux. | No payment handling, credential cloning, arbitrary card emulation, or access-control badge emulation. |
| legacy LF RFID | unpopulated 125 kHz evaluation footprint | Reserved only for an isolated manufacturing fixture decision after a threat model and compliance review. | Not fitted in the production design and no EM4100/HID Prox read, storage or emulation capability is planned. |

The STM32WB55RG selection matches the required dual-core layout and on-chip BLE capability; ST
lists a 64 MHz Cortex-M4, 32 MHz Cortex-M0+, 1 MB flash, 256 KB SRAM and BLE 5.4 support. The
ST25R3916 supports NFC-A/NFC-B reader operation; its broader card-emulation capability is
intentionally not exposed by this design. The CC1101 hardware covers the cited 300–348,
387–464 and 779–928 MHz ranges, but regional SKU, antenna matching and regulatory approval are
mandatory before even receive-only diagnostics ship. [STM32WB55RG product page](https://www.st.com/en/microcontrollers-microprocessors/stm32wb55rg.html), [ST25R3916 product page](https://www.st.com/en/nfc/st25r3916.html), [TI sub-1 GHz selector](https://www.ti.com/product-category/wireless-connectivity/sub-1-ghz/transceivers/products.html).

### 4.1 Electrical and software partition

```text
 Phone ── bonded BLE ──┐
 owner service NFC tag ├── STM32WB55 supervisor ── authenticated UART/SPI ── Linux compute
 emergency stop ───────┘              │                                      │
                                      └── motor-enable interlock              ├── robotd → motor bus
                                                                             ├── perception / brain
                                                                             └── mobile status API
```

- **Supervisor boot:** immutable boot path verifies signed STM32 firmware before enabling the
  Linux compute rail or motor-enable line. Linux releases cannot flash the supervisor.
- **Interlock protocol:** a versioned, CRC-protected command/status protocol carries only
  heartbeat, battery, stop reason, pairing state and approved tag assertion. It has no “raw
  transmit,” credential dump or actuator command.
- **RF coexistence:** put 2.4 GHz, 13.56 MHz and optional sub-GHz antennas on different faces of
  the torso, with keep-out zones from battery switching, servo harnesses, speaker magnet and the
  body IMU. Validate conducted/radiated emissions and coexistence before enclosure freeze.
- **Serviceability:** isolate the RF section on a shielded, replaceable supervisor board with
  keyed connectors, test pads inside the enclosure and an exterior regulatory/SKU label.

### 4.2 Mobile experience

The companion app should pair over the STM32 BLE service, then use the existing authenticated
LAN transport for larger state. Its “Brain” screen presents mobility-learning readiness, evidence
by cue, recent safety events, firmware trust state and battery condition. It may acknowledge a
service-tag scan or revoke a phone bond, but it cannot view, copy, replay or emulate radio/NFC/RFID
credentials. Pairing and status remain useful when Wi-Fi is unavailable; motion remains outside
the BLE control surface.

## 5. Software and policy boundary

MicroDuck 2.0 needs a distinct hardware profile, for example `microduck2`, with a new hardware
revision, kinematic chain, actuator map, calibration record, joint limits and policy-manifest
identity. The current 61-dimensional observation and 14-action policy contract is body-specific.
Existing ONNX policies must be rejected for the new body.

```text
CAD + measured actuator data
          │
          ▼
new URDF/MJCF + collision and mass model
          │
          ▼
simulation: stand → fall → recovery → low-speed walk → expression
          │
          ▼
new observation/action schema and `microduck2` policy manifest
          │
          ▼
robotd profile gate + bench bring-up + supervised hardware tests
```

`robotd` remains the sole motor owner. Its profile gate must reject a policy whose model, hardware
revision, actuator family, observation/action schema, control rate or calibration revision does
not match the attached body. The brain’s expressive requests remain advisory named skills and pass
the same gate.

The first executable digital-twin artifact is an original primitive MuJoCo reference body in the
`microduck-2.0-rl` repository: `robot/microduck2/robot.xml`, selected by
`duck-body --profile microduck2` (or `DUCK_SIM_PROFILE=microduck2 scripts/duck-sim`). It preserves
the wire protocol's 14 actuated-joint names and IMU/ToF sites, while modelling the 45 cm / 3.2 kg
reference envelope with broad feet and a 60 mm obstacle fixture. Its geometry, inertias, contact
parameters and actuator limits are provisional; it is a software-integration model, not a
sim-to-real claim or a trainable hardware policy model until CAD and bench data replace them.

## 6. Larger-body language

The added scale buys legibility, not faster movement. Start with slow, bounded expressions:

| expression | motion | safety boundary |
| --- | --- | --- |
| curious tilt | head/neck inclination with a small weight shift | feet stay planted; stance policy owns balance |
| greeting bow | torso/head pitch and controlled return | no forward foot travel; cancel on human control or contact fault |
| happy bob | small vertical body motion and head rhythm | amplitude/cadence limited by validated stance margin |
| settle | lower-energy posture, gaze down and quiet idle motion | never a torque-off or collapse command |
| yield | expression fades and control visibly returns to the person | no autonomous movement while human control is active |

Each is a separate, time-bounded 2.0 policy with entry/exit poses, interruption behavior,
contact/fall cases, thermal budget, simulation evidence and hardware rehearsal. No motion policy
is represented as complete until that evidence exists.

## 7. Staged build and acceptance gates

1. **Envelope and risk review:** freeze height, mass, speed, environment and transport
   assumptions; deliver CAD mass properties and preliminary hazard analysis.
2. **Passive mule:** dummy-actuator frame validates reach, centre of mass, cable routing,
   clearances, carrying points and controlled tip-over plan before powered movement.
3. **One-leg bench rig:** guarded, instrumented actuator characterization establishes thermal
   limits, backlash, torque-off behavior and hard stops before ordering a full robot.
4. **Digital twin:** validate the MJCF/URDF, contact, joint-limit, inertia and actuator models;
   retrain standing, recovery, walking and expressive policies.
5. **Tethered bring-up:** read sensors, verify direction/limits, demonstrate torque-off, then
   static stance with a physically reachable emergency stop.
6. **Supervised locomotion:** low-speed guarded walking/recovery with measured temperatures,
   current, timing and falls. Brain-triggered skills remain disabled.
7. **RF/security qualification:** verify BLE bond revocation, signed supervisor update, UART/SPI
   message rejection, NFC signed-tag verification, antenna coexistence and regional RF SKU
   configuration. The optional CC1101 board stays receive-only during this gate.
8. **Expression release:** enable one short expression after simulation-to-hardware comparison and
   interruption tests. Default the brain to observe-only until each expression is approved.

There is no full-size release gate without recorded fall, pinch, thermal, battery, actuator,
emergency-stop and recovery testing.

## 8. Open decisions

- One 18 in reference chassis first, versus 12–15 in and 20–24 in variants.
- Biped only or an optional rolling module; a rolling module is a distinct mechanical and policy
  profile, not an accessory that shares walking calibration.
- Exact actuator family, battery chemistry/capacity and payload, pending calculation and testing.
- Whether the head carries only expression hardware or an arm/gripper; a gripper is out of the
  first chassis because it changes collision, pinch and centre-of-mass requirements.
- A perception/privacy design for consented local “person nearby” or touch evidence. Until then,
  the brain is limited to direct-control and idle stimuli.
- Whether the optional receive-only CC1101 diagnostic daughterboard has enough legitimate support
  value to justify regional certification, antenna validation and enclosure cost.
- The signed owner/service NFC tag format, lifecycle, revocation flow and recovery path; it must
  be independent of any building or vehicle access credential.
