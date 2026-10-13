//! What a gamepad means to a duck: pad state in, intents out.
//!
//! Moved out of `padd` so that `netpadd` — the same pad, arriving over UDP — drives the robot
//! with exactly the same mapping rather than a copy of it that drifts. It is pure: no socket, no
//! clock of its own, no config file read behind the caller's back. The two daemons own the I/O;
//! this owns what a button means. `padd`'s crate doc still describes the mapping itself.

mod bindings;
mod buttons;
mod continuous;
pub mod handover;
mod hold;
pub mod loss;
mod mapper;
pub mod wire;

pub use bindings::{BINDINGS_POLL, Overrides, parse_deadzone, parse_max_head};
pub use buttons::Buttons;
pub use continuous::{Continuous, HEARTBEAT};
pub use hold::{HoldAction, HoldButton};
pub use loss::{PadLoss, PadLossAction};
pub use mapper::{Config, Mapper, Out, PadFrame, refusal, report};

/// Whether `[netpad] enabled` runs `netpadd` — its unit's `ExecCondition=`. `padd` always runs;
/// [`handover`] is how the two share the robot.
///
/// A file that cannot be read answers **no**: a bad config never opens the UDP port.
pub fn udp_selected(config: &std::path::Path) -> bool {
    match robotd_params::Params::load(config, false) {
        Ok(params) => params.netpad.enabled,
        Err(e) => {
            tracing::warn!(error = %e, "cannot read the config; the UDP pad stays off");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_selector_reads_netpad_enabled_and_a_bad_file_keeps_udp_off() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("robotd.toml");
        std::fs::write(&p, "[netpad]\nenabled = true\n").unwrap();
        assert!(super::udp_selected(&p));
        std::fs::write(&p, "[netpad]\nenabled = false\n").unwrap();
        assert!(!super::udp_selected(&p));
        std::fs::write(&p, "[netpad\n").unwrap();
        assert!(!super::udp_selected(&p), "unreadable → off");
        assert!(
            !super::udp_selected(&dir.path().join("missing.toml")),
            "absent → defaults → off"
        );
    }
}
