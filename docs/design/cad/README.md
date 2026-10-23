# MicroDuck 2.0 CAD reference package

`microduck2_reference.scad` is the reviewable, parametric envelope model for the 45 cm reference
body. It has a switchable assembly, torso, leg, foot, head and electronics-tray view. It is useful
for checking scale, clearances, component placement and early enclosure discussions in OpenSCAD.

It is **not a fabrication release**. The geometry uses conservative primitives because there is no
approved actuator family, fastener stack, material specification, wiring harness, CAD mass model,
FEA result or physical qualification. The companion MuJoCo body is equally an integration model;
neither authorizes a build or a locomotion policy.

Before generating manufacturing drawings or STL/STEP releases, complete this package:

1. Select and bench-characterize the actuator and reduction system.
2. Replace every primitive with source-controlled CAD assemblies, mass properties and collision
   geometry.
3. Freeze battery chemistry, BMS, fusing, service disconnect, emergency-stop and enclosure ingress
   design.
4. Complete an electrical schematic/PCB layout with antenna keep-outs, regulatory review and EMC
   test plan.
5. Produce drawing sheets with material, finish, tolerance, fastener torque, cable route and
   inspection requirements.
6. Re-export the physical MuJoCo model and retrain all 2.0 policies from scratch.

`BOM-preliminary.csv` deliberately labels unresolved items instead of inventing purchasable parts.
