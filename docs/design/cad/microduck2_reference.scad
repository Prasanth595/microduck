/*
 * MicroDuck 2.0 reference CAD
 * SPDX-License-Identifier: Apache-2.0
 *
 * Parametric envelope and service-clearance model for the 450 mm reference chassis.
 * This is deliberately NOT production geometry: it cannot be used to manufacture a walking robot
 * until the structural frame, actuator selection, fasteners and every inertia value are approved.
 * Open in OpenSCAD; set `part` to export an individual module for review.
 */

part = "assembly"; // [assembly:torso:left_leg:right_leg:foot:head:electronics_tray]

// ── design envelope, all dimensions mm ──────────────────────────────────────
standing_height = 450;
torso_l = 240;
torso_w = 170;
torso_h = 190;
hip_height = 285;
thigh_l = 150;
shin_l = 150;
foot_l = 210;
foot_w = 120;
foot_h = 45;
wall = 3.0;
clearance = 1.0;
mount_pitch = 50;

assert(standing_height >= 380 && standing_height <= 500, "Reference height must remain 380–500 mm");
assert(foot_w >= 110, "Foot support polygon is below the approved preliminary minimum");

$fn = 48;

module rounded_box(size, radius=10) {
  hull() for (x=[radius, size[0]-radius], y=[radius, size[1]-radius], z=[radius, size[2]-radius])
    translate([x,y,z]) sphere(radius);
}

module hole(d=4, h=20) { cylinder(d=d, h=h, center=true); }

module torso_shell() {
  difference() {
    color("gold") rounded_box([torso_l, torso_w, torso_h], 24);
    translate([wall,wall,wall]) rounded_box([torso_l-2*wall,torso_w-2*wall,torso_h], 21);
    // Removable electronics/service hatch on the back face.
    translate([torso_l-45, torso_w/2-45, torso_h/2]) cube([60,90,110], center=true);
    // Four supervisor-board standoffs: 50 mm square, M2.5 clearance.
    for (y=[torso_w/2-mount_pitch/2, torso_w/2+mount_pitch/2], z=[70,120])
      translate([20,y,z]) rotate([0,90,0]) hole(2.8, 20);
  }
}

module pelvis_frame() {
  color("dimgray") translate([35, 15, 0]) difference() {
    cube([150,140,20]);
    translate([25,20,-1]) cube([100,100,24]);
    for (y=[25,115]) translate([75,y,10]) hole(5,30);
  }
}

module leg(side=1) {
  y = side * (foot_w/2 - 22);
  // Segment axes are vertical in the neutral inspection pose. Actual joint stack is pending CAD.
  color("silver") translate([85,y,hip_height-thigh_l]) cube([34,44,thigh_l]);
  color("silver") translate([85,y,hip_height-thigh_l-shin_l]) cube([30,40,shin_l]);
  color("black") translate([102,y,hip_height]) rotate([90,0,0]) cylinder(d=58,h=54,center=true);
  color("black") translate([100,y,hip_height-thigh_l]) rotate([90,0,0]) cylinder(d=52,h=50,center=true);
  color("black") translate([100,y,hip_height-thigh_l-shin_l]) rotate([90,0,0]) cylinder(d=48,h=48,center=true);
  translate([0, y-foot_w/2+22, hip_height-thigh_l-shin_l-foot_h]) foot();
}

module foot() {
  difference() {
    color("goldenrod") rounded_box([foot_l,foot_w,foot_h], 14);
    // Compliant sole pocket; leaves a 3 mm skin for an interchangeable tread.
    translate([8,8,-1]) rounded_box([foot_l-16,foot_w-16,foot_h-3], 10);
    // Toe/heel load-cell cavity; routing, connector and actual sensor are not yet specified.
    for (x=[45,165]) translate([x,foot_w/2,foot_h/2]) hole(8, foot_h+2);
  }
}

module head() {
  color("gold") translate([85,torso_w/2,torso_h+85]) scale([1.25,1,1]) sphere(68);
  color("orange") translate([175,torso_w/2,torso_h+78]) rotate([0,90,0]) cylinder(r1=18,r2=7,h=55);
  // Head NFC loop and BLE antenna keep-out are indicated, not conductive geometry.
  color("deepskyblue", 0.35) translate([100,torso_w/2,torso_h+85]) rotate([90,0,0]) cylinder(d=58,h=2,center=true);
}

module electronics_tray() {
  color("seagreen") translate([20,35,35]) cube([110,100,3]);
  // Keep RF parts away from power and motor harness. Blue = STM32WB supervisor, purple = NFC.
  color("royalblue") translate([35,55,39]) cube([50,50,12]);
  color("purple") translate([105,55,39]) cube([28,50,8]);
}

module assembly() {
  torso_shell(); pelvis_frame(); electronics_tray();
  translate([0,0,0]) leg(1); leg(-1); head();
  // Floor and height datum make the 450 mm review envelope unambiguous.
  color([0.18,0.2,0.24,0.35]) translate([-60,-110,0]) cube([360,390,3]);
  color("red") translate([280,0,0]) cube([2,2,standing_height]);
}

if (part == "assembly") assembly();
else if (part == "torso") torso_shell();
else if (part == "left_leg") leg(1);
else if (part == "right_leg") leg(-1);
else if (part == "foot") foot();
else if (part == "head") head();
else if (part == "electronics_tray") electronics_tray();
