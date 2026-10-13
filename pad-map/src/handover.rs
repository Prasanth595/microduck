//! Two pad daemons, one robot: which of them is driving, and how each tells the other.
//!
//! `padd` (the Bluetooth pad) always runs; `netpadd` (a pad over UDP) runs too while
//! `[netpad] enabled` is on. Each sends nothing while it has no pad, so most of the time only one
//! of them is talking to `robotd` and there is nothing to decide. When both have one, **the
//! Bluetooth pad wins**: it was paired by somebody holding it next to the robot, and the UDP port
//! is unauthenticated — whoever is standing by the duck with a pad in their hand must always be
//! able to take it back from the network. `netpadd` answers its client with
//! [`crate::wire::Verdict::Bluetooth`] and sends nothing until the Bluetooth pad goes away.
//!
//! Each daemon says it is driving with a file in its own runtime directory, present exactly while
//! it has a pad. A file and not a socket, because the question is one bit and the answer has to
//! survive the asker restarting; and the runtime directory because systemd removes it whenever the
//! unit stops, crashed or not, so a dead daemon can never be left claiming the robot.
//!
//! The other direction matters too: a pad that goes away sits the robot a second later
//! ([`crate::loss`]). When the other daemon is driving by then, the robot was handed over, not
//! abandoned, and the sit is called off.

use std::path::{Path, PathBuf};

/// Present while `padd` has a Bluetooth pad.
pub const BLUETOOTH: &str = "/run/padd/driving";
/// Present while `netpadd` has a UDP client.
pub const UDP: &str = "/run/netpadd/driving";

/// Is the daemon that owns `marker` driving? A stat, cheap enough to ask every tick.
pub fn driving(marker: &Path) -> bool {
    marker.exists()
}

/// This daemon's own marker, written only on a change.
#[derive(Debug)]
pub struct Marker {
    path: PathBuf,
    on: bool,
}

impl Marker {
    /// Removes a marker left by an earlier run of the same daemon outside its unit; under systemd
    /// the runtime directory is fresh and this finds nothing.
    pub fn new(path: PathBuf) -> Self {
        let _ = std::fs::remove_file(&path);
        Self { path, on: false }
    }

    /// Say whether this daemon is driving. Best effort: a marker that cannot be written costs the
    /// other daemon the hand-over, never this one its pad, so it is logged and stepped over.
    pub fn set(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        self.on = on;
        let result = if on {
            std::fs::write(&self.path, b"")
        } else {
            std::fs::remove_file(&self.path)
        };
        if let Err(e) = result {
            tracing::warn!(
                error = %e, path = %self.path.display(),
                "cannot update the driving marker — the other pad daemon will not see this one"
            );
        }
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        if self.on {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_is_there_exactly_while_driving() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("driving");
        std::fs::write(&path, b"").unwrap();
        let mut m = Marker::new(path.clone());
        assert!(!driving(&path), "a leftover from an earlier run is cleared");
        m.set(true);
        assert!(driving(&path));
        m.set(true);
        assert!(driving(&path));
        m.set(false);
        assert!(!driving(&path));
        m.set(true);
        drop(m);
        assert!(
            !driving(&path),
            "a daemon that ends stops claiming the robot"
        );
    }
}
