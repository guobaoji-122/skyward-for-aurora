//! Read the phone's contacts and say what it finds, so that the reading can
//! be tried on the phone itself.
//!
//!     reads            the contacts the provider keeps
//!     reads calls      the calls the provider recorded

use anyhow::Result;
use skyward_contacts::{Calls, Contacts, CALLS, DATABASE};

fn main() -> Result<()> {
    let what = std::env::args().nth(1).unwrap_or_default();
    if what == "calls" {
        let calls = Calls::open(std::path::Path::new(CALLS))?;
        let calls = calls.recent(20)?;
        println!("{} calls", calls.len());
        for call in calls {
            println!("  {} {} {}s", call.number, call.at, call.seconds);
        }
        return Ok(());
    }
    let contacts = Contacts::open(std::path::Path::new(DATABASE))?;
    let people = contacts.all()?;
    println!("{} people", people.len());
    for person in people {
        println!("  {} — {}", person.name, person.numbers.join(", "));
    }
    Ok(())
}
