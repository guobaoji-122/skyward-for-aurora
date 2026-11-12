//! Read the phone's contacts and calls, using the provider's own library, so
//! the reading tried on the phone is the reading the recovery will do.
//!
//!     reads                 the contacts the provider keeps
//!     reads calls           the calls the provider recorded
//!     reads [calls] <path>  read the database at <path> instead
//!
//! The database is opened the way the library opens it: a read-only URI that
//! tells SQLite nothing will change it.

use anyhow::Result;
use skyward_contacts::{Calls, Contacts, CALLS, DATABASE};
use std::path::Path;

fn main() -> Result<()> {
    let what = std::env::args().nth(1).unwrap_or_default();
    let path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| if what == "calls" { CALLS } else { DATABASE }.to_string());
    let path = Path::new(&path);
    eprintln!("path: {}", path.display());

    if what == "calls" {
        let calls = Calls::open(path)?;
        let recent = calls.recent(20)?;
        for call in &recent {
            eprintln!("  {} {}s kind={}", call.number, call.seconds, call.kind);
        }
        eprintln!("{} calls", recent.len());
    } else {
        let contacts = Contacts::open(path)?;
        let all = contacts.all()?;
        for contact in &all {
            eprintln!("  {} — {}", contact.name, contact.numbers.join(", "));
        }
        eprintln!("{} people", all.len());
    }
    Ok(())
}
