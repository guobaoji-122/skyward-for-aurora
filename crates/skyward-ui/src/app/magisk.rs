//! What the Magisk page shows, as the runtime reads it.

use super::*;
use crate::magisk::{MagiskModule, MagiskRoot};

impl App {
    /// The runtime read the root solution and its modules.
    pub fn magisk_read(&mut self, root: MagiskRoot, modules: Result<Vec<MagiskModule>, String>) {
        self.state.magisk.root = root;
        self.state.magisk.modules = Some(modules);
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

    /// A module was switched or marked for removal.
    pub fn magisk_changed(&mut self, ended: Result<(), String>, now: u64) {
        match ended {
            Ok(()) => self.refresh(),
            Err(why) => self.say(&why, now),
        }
    }
}
