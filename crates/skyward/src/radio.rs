//! What the phone's radio is doing, said in words.
//!
//! The phone's radio is a set of services the phone's own framework
//! talks to over the binder. Before anything can be asked of them, they
//! have to be registered with the phone's service directory, and that is
//! the first thing this looks at: a phone whose radios are not registered
//! says nothing about its network, and the reason why they did not come
//! up is in the kernel log.
//!
//! What this says goes to the kernel log, so that it is there to be read
//! after the fact rather than only on a screen. The command that asks
//! for it prints it as well.

use anyhow::{Context, Result};
use skyward_binder::driver::Binder;
use std::time::{Duration, Instant};

/// How long to wait between the looks at the service directory.
const POLL: Duration = Duration::from_millis(500);

/// Where the words go.
enum Where {
    /// The kernel log, which is kept after the recovery ends.
    Log,
    /// The screen, for a person who asked.
    Screen,
}

impl Where {
    fn say(&self, line: &str) {
        match self {
            Where::Log => {
                let _ = skyward_sys::kmsg(line);
            }
            Where::Screen => println!("{line}"),
        }
    }
}

/// Say what the phone's radio is doing, in the kernel log.
pub fn report(wait: u64) -> Result<()> {
    say(wait, Where::Log)
}

/// Say what the phone's radio is doing, on the screen.
pub fn print(wait: u64) -> Result<()> {
    say(wait, Where::Screen)
}

/// Say what the phone's radio is doing.
fn say(wait: u64, where_: Where) -> Result<()> {
    let binder = Binder::open().context("reach the phone's binder")?;
    let until = Instant::now() + Duration::from_secs(wait);

    let mut cards = skyward_radio::cards(&binder).context("ask the phone what has registered")?;
    while cards.is_empty() && Instant::now() < until {
        std::thread::sleep(POLL);
        cards = skyward_radio::cards(&binder).context("ask the phone what has registered")?;
    }

    if cards.is_empty() {
        where_.say("no radio has registered with the phone; the radio's own daemon has not come up");
        for (name, up) in skyward_radio::programs() {
            where_.say(&format!("{name} is {}", if up { "running" } else { "not running" }));
        }
        return Ok(());
    }

    where_.say(&format!("the phone has {} card(s) with a radio", cards.len()));
    for (name, up) in skyward_radio::programs() {
        where_.say(&format!("{name} is {}", if up { "running" } else { "not running" }));
    }
    for slot in &cards {
        let radio = skyward_radio::Radio::open(&binder, *slot)?;
        let names = radio.names();
        if names.is_empty() {
            where_.say(&format!("slot {slot}: nothing has answered"));
            continue;
        }
        where_.say(&format!("slot {slot}: {}", names.join(", ")));
        // The radio's own configuration is the one that answers at once,
        // so it is the one that can be asked without waiting for anything.
        match radio.configuration() {
            Ok(c) if !c.version.is_empty() => where_.say(&format!("slot {slot}: the radio is {}", c.version)),
            Ok(_) => {}
            Err(e) => where_.say(&format!("slot {slot}: the radio did not say its version: {e:#}")),
        }
    }
    Ok(())
}
