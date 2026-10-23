# MicroDuck 2.0 proposal and fork policy

MicroDuck 2.0 is intended as an Apache-2.0 open-source derivative of
[`pollen-robotics/microduck`](https://github.com/pollen-robotics/microduck). Preserve upstream
copyright notices, the Apache-2.0 license, attribution in this file and an obvious distinction
between upstream MicroDuck and the proposed `microduck2` hardware family.

## Recommended repository arrangement

Create an organization-owned fork named `Magnexis/microduck` to retain a visible relationship with
upstream. Develop 2.0 on the `microduck2` branch and add the sister RL fork as
`Magnexis/microduck-rl`, also on a `microduck2` branch. Do not rename the upstream project in the
fork metadata; use the description “Magnexis proposal: MicroDuck 2.0 larger-body research branch.”

Keep `upstream` pointed at `https://github.com/pollen-robotics/microduck.git` and keep the fork's
default branch aligned with upstream. The `microduck2` branch then carries the new CAD, simulator,
hardware-profile and brain work without implying that it is maintained or endorsed by Pollen
Robotics.

## Before publishing

- Confirm the exact Magnexis GitHub organization slug and that it has a GitHub App installation
  authorized to create repositories/forks.
- Review every uncommitted change, remove generated binaries and verify all license notices.
- Add `UPSTREAM.md` with the pinned upstream revision and a per-directory provenance table.
- Replace concept CAD and the primitive MuJoCo body with reviewed engineering artifacts before
  describing the project as build-ready.
- Open a draft proposal issue or discussion upstream only after the public fork is available; link
  it as an invitation for feedback, not as an endorsement claim.
