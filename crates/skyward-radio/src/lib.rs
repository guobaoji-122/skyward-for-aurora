//! The phone's radio, as the recovery asks it.
//!
//! The radio of a modern Android phone is a set of services the framework
//! talks to over the binder: one for the network, one for the SIM, one for
//! the modem, one for calls. Each is named after its interface and the
//! card it serves, so a phone with two cards has two of each.
//!
//! Every one of those interfaces is *oneway*: a method takes a number and
//! returns nothing, and the answer comes later through a callback object
//! the phone's own framework registers with [`set_response_functions`].
//! Reading the radio therefore takes two steps -- registering the object
//! the answers arrive at, and then asking -- and the answer arrives on a
//! thread of its own.
//!
//! The methods are numbered in the order their interface declares them, and
//! that order is what the numbers below are: the eleventh method of the
//! network interface is the one that asks for the signal strength.
//!
//! What is here that does not need the phone is the numbering itself, the
//! reading of the names, and the telling of the four Chinese networks
//! apart, so that all of it can be checked on a computer.

pub mod carrier;
pub mod signal;

use anyhow::{Context, Result};
use skyward_binder::parcel::Parcel;

#[cfg(target_os = "android")]
use skyward_binder::driver::Binder;
#[cfg(target_os = "android")]
use skyward_binder::service;

/// The interface of the network radio.
pub const NETWORK: &str = "android.hardware.radio.network.IRadioNetwork";
/// The interface of the SIM radio.
pub const SIM: &str = "android.hardware.radio.sim.IRadioSim";
/// The interface of the modem radio.
pub const MODEM: &str = "android.hardware.radio.modem.IRadioModem";
/// The interface of the voice radio.
pub const VOICE: &str = "android.hardware.radio.voice.IRadioVoice";
/// The interface of the radio's own configuration, which is not a radio of
/// its own but says what the phone has.
pub const CONFIG: &str = "android.hardware.radio.config.IRadioConfig";

/// The methods of the radio's configuration, in the order it declares
/// them. This interface answers at once, without a callback: it is the
/// one way to ask the radio something and be told straight back.
pub mod config {
    /// Ask how many radios the phone has.
    pub const MODEM_COUNT: u32 = 1;
    /// Ask which radio the phone prefers.
    pub const PREFERRED_RADIO_MODEM: u32 = 2;
    /// Ask what the phone's radios are.
    pub const RADIO_CAPABILITIES: u32 = 3;
    /// Ask which radios the phone has.
    pub const RADIO_INFOS: u32 = 4;
    /// Ask the radio's own version.
    pub const RADIO_VERSION: u32 = 5;
}

/// The methods of the network interface, in the order it declares them.
pub mod network {
    /// Ask which networks the phone may use.
    pub const ALLOWED_NETWORK_TYPES: u32 = 1;
    /// Ask which bands the radio has.
    pub const AVAILABLE_BAND_MODES: u32 = 2;
    /// Ask which networks can be seen.
    pub const AVAILABLE_NETWORKS: u32 = 3;
    /// Ask what bars the phone is under.
    pub const BARRING_INFO: u32 = 4;
    /// Ask which networks the phone is on.
    pub const DATA_REGISTRATION_STATE: u32 = 7;
    /// Ask how the phone is registered for IMS.
    pub const IMS_REGISTRATION_STATE: u32 = 8;
    /// Ask whether the phone picks its network itself.
    pub const NETWORK_SELECTION_MODE: u32 = 9;
    /// Ask who the phone's network belongs to.
    pub const OPERATOR: u32 = 10;
    /// Ask how strong the signal is.
    pub const SIGNAL_STRENGTH: u32 = 11;
    /// Ask which radio technology the phone's calls use.
    pub const VOICE_RADIO_TECHNOLOGY: u32 = 13;
    /// Ask whether the phone has a network to call on.
    pub const VOICE_REGISTRATION_STATE: u32 = 14;
    /// Tell the phone the answers were heard.
    pub const RESPONSE_ACKNOWLEDGEMENT: u32 = 16;
    /// Register the objects the answers arrive at.
    pub const SET_RESPONSE_FUNCTIONS: u32 = 28;
}

/// The methods of the SIM interface, in the order it declares them.
pub mod sim {
    /// Ask which carriers the card may be used with.
    pub const ALLOWED_CARRIERS: u32 = 5;
    /// Ask what the card says about itself.
    pub const ICC_CARD_STATUS: u32 = 9;
    /// Ask for the card's number.
    pub const IMSI_FOR_APP: u32 = 10;
    /// Register the objects the answers arrive at.
    pub const SET_RESPONSE_FUNCTIONS: u32 = 28;
}

/// The methods of the modem interface, in the order it declares them.
pub mod modem {
    /// Ask what the modem's own version is.
    pub const BASEBAND_VERSION: u32 = 2;
    /// Ask whether the modem is ready.
    pub const MODEM_STACK_STATUS: u32 = 6;
    /// Register the objects the answers arrive at.
    pub const SET_RESPONSE_FUNCTIONS: u32 = 17;
}

/// The methods of the voice interface, in the order it declares them.
pub mod voice {
    /// Register the objects the answers arrive at.
    pub const SET_RESPONSE_FUNCTIONS: u32 = 39;
}

/// What the phone's network radio says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Network {
    /// Whether the phone has a network registered on it at all.
    pub registered: bool,
    /// Which of the states the network can be in.
    pub registration: i32,
    /// The name of the network's owner, as the phone spells it.
    pub operator_name: String,
    /// The network's number, as the phone spells it.
    pub operator_numeric: String,
    /// The country the network belongs to.
    pub operator_country: String,
    /// The signal strength, in the phone's own scale, and the technology
    /// the signal is carried on.
    pub signal: Option<Signal>,
}

/// The strength of the signal the phone can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    /// The strength as the phone counts it, from 0 to 31.
    pub level: i32,
    /// The radio technology the signal comes on, as the phone names it.
    pub technology: String,
}

/// What the phone's SIM radio says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sim {
    /// Whether a card is in the phone at all.
    pub present: bool,
    /// Whether the card is locked, and by what.
    pub locked: bool,
    /// The state of the card, as the phone numbers it.
    pub state: i32,
}

/// What the phone's modem radio says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Modem {
    /// The modem's own version.
    pub baseband: String,
    /// Whether the modem is up.
    pub ready: bool,
}

/// The registration states the phone's network radio answers with, as the
/// platform numbers them.
pub mod registration {
    /// Not registered at all.
    pub const NOT_REGISTERED: i32 = 0;
    /// Searching for a network.
    pub const SEARCHING: i32 = 1;
    /// Registered, but not on the phone's home network.
    pub const REGISTERED_ROAMING: i32 = 2;
    /// Registered on the home network.
    pub const REGISTERED_HOME: i32 = 3;
}

/// The card a radio name serves, when the name is one of the radio's.
pub fn slot_of(name: &str) -> Option<u32> {
    for interface in [NETWORK, SIM, MODEM, VOICE] {
        if let Some(rest) = name.strip_prefix(interface) {
            if let Some(slot) = rest.strip_prefix("/slot") {
                if let Ok(slot) = slot.parse::<u32>() {
                    return Some(slot);
                }
            }
        }
    }
    None
}

/// The programs the radio is made of, and whether each is running.
///
/// The radio of a phone is two programs: one that loads the modem's
/// firmware into the modem, and one that answers when the radio is asked
/// anything. A radio that has not come up has failed in one of them, and
/// which one is the difference between a modem that is not there and a
/// daemon that did not start.
pub const PROGRAMS: [&str; 3] = ["rmt_storage", "qcrilNrd", "ntn_rild"];

/// Whether the programs the radio is made of are running.
pub fn programs() -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for name in PROGRAMS {
        out.push((name.to_string(), running(name)));
    }
    out
}

/// Whether a program of that name is running, by what the process list says.
fn running(name: &str) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else { return false };
    for entry in entries.flatten() {
        let file = entry.path();
        let Some(pid) = file.file_name().and_then(|n| n.to_str()) else { continue };
        if !pid.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(cmdline) = std::fs::read(file.join("cmdline")) else { continue };
        // The command line is a run of names, each ending in a null.
        let first = cmdline.split(|b| *b == 0).next().unwrap_or(&[]);
        let first = String::from_utf8_lossy(first);
        let base = first.rsplit('/').next().unwrap_or(&first);
        if base == name {
            return true;
        }
    }
    false
}

/// The number a request carries, so that the phone can tell the answers
/// apart. Each call takes the next number.
pub fn next_serial(serial: &mut i32) -> i32 {
    *serial += 1;
    *serial
}

/// A call to a radio method, with the interface and the number it carries.
fn call(interface: &str, serial: i32) -> Parcel {
    let mut p = Parcel::call(interface);
    p.i32(serial);
    p
}

/// The radio of one card.
#[cfg(target_os = "android")]
pub struct Radio<'a> {
    network: Option<skyward_binder::driver::Handle<'a>>,
    sim: Option<skyward_binder::driver::Handle<'a>>,
    modem: Option<skyward_binder::driver::Handle<'a>>,
    config: Option<skyward_binder::driver::Handle<'a>>,
}

#[cfg(target_os = "android")]
impl<'a> Radio<'a> {
    /// The radio of the card in `slot`, which the phone numbers from one.
    /// A card the phone has no radio for is not an error: the phone may
    /// have one card and two radios, or the other way about.
    pub fn open(binder: &'a Binder, slot: u32) -> Result<Radio<'a>> {
        Ok(Radio {
            network: find(binder, NETWORK, slot)?,
            sim: find(binder, SIM, slot)?,
            modem: find(binder, MODEM, slot)?,
            config: service::find(binder, CONFIG).context("look up the radio's configuration")?,
        })
    }

    /// Whether any radio at all answered for this card.
    pub fn present(&self) -> bool {
        self.network.is_some() || self.sim.is_some() || self.modem.is_some() || self.config.is_some()
    }

    /// What the radios of this card are called, for the log.
    pub fn names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (what, found) in [
            ("network", &self.network),
            ("sim", &self.sim),
            ("modem", &self.modem),
            ("config", &self.config),
        ] {
            if found.is_some() {
                out.push(what.to_string());
            }
        }
        out
    }

    /// The network radio of this card, or what is wrong with it.
    pub fn network(&self) -> Result<&skyward_binder::driver::Handle<'a>> {
        self.network.as_ref().with_context(|| "this card has no network radio")
    }

    /// The SIM radio of this card, or what is wrong with it.
    pub fn sim(&self) -> Result<&skyward_binder::driver::Handle<'a>> {
        self.sim.as_ref().with_context(|| "this card has no SIM radio")
    }

    /// The modem radio of this card, or what is wrong with it.
    pub fn modem(&self) -> Result<&skyward_binder::driver::Handle<'a>> {
        self.modem.as_ref().with_context(|| "this card has no modem radio")
    }

    /// The radio's own configuration, which answers at once.
    pub fn config(&self) -> Result<&skyward_binder::driver::Handle<'a>> {
        self.config.as_ref().with_context(|| "the phone has no radio configuration")
    }

    /// What the radio's own configuration says about the phone.
    ///
    /// This is the one question that can be asked and answered without a
    /// callback object: the radio's other interfaces answer only later,
    /// through an object of the caller's own, and this one answers at once.
    /// What it says is therefore the first thing to learn about a radio
    /// that has come up.
    pub fn configuration(&self) -> Result<Configuration> {
        let handle = self.config()?;
        let p = Parcel::call(CONFIG);
        let reply = handle.transact(config::RADIO_VERSION, &p).context("ask the radio for its version")?;
        let mut read = reply.read();
        skyward_binder::status::ok(&mut read).context("ask the radio for its version")?;
        Ok(Configuration {
            version: read.string().unwrap_or_default(),
        })
    }
}

/// What the radio's own configuration says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Configuration {
    /// The radio's version, as the phone spells it.
    pub version: String,
}

/// The radio of the card in `slot`, as the phone's service directory has it.
#[cfg(target_os = "android")]
fn find<'a>(binder: &'a Binder, interface: &str, slot: u32) -> Result<Option<skyward_binder::driver::Handle<'a>>> {
    let name = format!("{interface}/slot{slot}");
    service::find(binder, &name).with_context(|| format!("look up {name}"))
}

/// Whether the phone's radios are running at all, and which cards they
/// serve. This is the first thing to ask: a phone whose radios are not up
/// says nothing about its network, and the reason why belongs in the log.
#[cfg(target_os = "android")]
pub fn cards(binder: &Binder) -> Result<Vec<u32>> {
    let names = service::names(binder).context("list the phone's services")?;
    let mut out = Vec::new();
    for name in &names {
        if let Some(slot) = slot_of(name) {
            if !out.contains(&slot) {
                out.push(slot);
            }
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// Ask a radio a question. The answer does not come back here: the phone
/// sends it to the object registered for it, which is what
/// [`set_response_functions`] is for.
#[cfg(target_os = "android")]
pub fn ask(handle: &skyward_binder::driver::Handle, interface: &str, method: u32, serial: i32) -> Result<()> {
    let parcel = call(interface, serial);
    handle.transact(method, &parcel).context("ask the radio a question")?;
    Ok(())
}

/// Ask the network radio how the signal is.
#[cfg(target_os = "android")]
pub fn signal_strength(handle: &skyward_binder::driver::Handle, serial: i32) -> Result<()> {
    ask(handle, NETWORK, network::SIGNAL_STRENGTH, serial)
}

/// Ask the network radio who the phone's network belongs to.
#[cfg(target_os = "android")]
pub fn operator(handle: &skyward_binder::driver::Handle, serial: i32) -> Result<()> {
    ask(handle, NETWORK, network::OPERATOR, serial)
}

/// Ask the network radio whether the phone has a network registered.
#[cfg(target_os = "android")]
pub fn voice_registration(handle: &skyward_binder::driver::Handle, serial: i32) -> Result<()> {
    ask(handle, NETWORK, network::VOICE_REGISTRATION_STATE, serial)
}

/// Ask the SIM radio what the card says about itself.
#[cfg(target_os = "android")]
pub fn sim_status(handle: &skyward_binder::driver::Handle, serial: i32) -> Result<()> {
    ask(handle, SIM, sim::ICC_CARD_STATUS, serial)
}

/// Ask the modem radio what version it is.
#[cfg(target_os = "android")]
pub fn baseband_version(handle: &skyward_binder::driver::Handle, serial: i32) -> Result<()> {
    ask(handle, MODEM, modem::BASEBAND_VERSION, serial)
}

/// Tell the phone's radio where its answers should go.
///
/// The phone's framework does this with two objects of its own: one the
/// answers arrive at, and one the phone's own events are sent to. The
/// recovery has neither, so it makes its own.
#[cfg(target_os = "android")]
pub fn set_response_functions(
    handle: &skyward_binder::driver::Handle,
    interface: &str,
    method: u32,
    response: i32,
    indication: i32,
) -> Result<()> {
    let mut p = Parcel::call(interface);
    p.i32(response);
    p.i32(indication);
    handle.transact(method, &p).context("tell the radio where its answers go")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cards_are_told_apart_by_their_names() {
        assert_eq!(slot_of("android.hardware.radio.network.IRadioNetwork/slot1"), Some(1));
        assert_eq!(slot_of("android.hardware.radio.sim.IRadioSim/slot2"), Some(2));
        assert_eq!(slot_of("android.hardware.radio.modem.IRadioModem/slot1"), Some(1));
        assert_eq!(slot_of("android.hardware.radio.voice.IRadioVoice/slot2"), Some(2));
        assert_eq!(slot_of("android.hardware.security.keymint.IKeyMintDevice/default"), None);
        assert_eq!(slot_of("android.hardware.radio.network.IRadioNetwork"), None);
    }

    #[test]
    fn each_question_carries_a_number_of_its_own() {
        let mut serial = 0;
        assert_eq!(next_serial(&mut serial), 1);
        assert_eq!(next_serial(&mut serial), 2);
    }

    #[test]
    fn the_method_numbers_are_the_ones_the_interface_declares() {
        // The network interface's eleventh method asks for the signal
        // strength, its tenth for the operator, and its fourteenth for
        // whether the phone is registered.
        assert_eq!(network::SIGNAL_STRENGTH, 11);
        assert_eq!(network::OPERATOR, 10);
        assert_eq!(network::VOICE_REGISTRATION_STATE, 14);
        assert_eq!(network::DATA_REGISTRATION_STATE, 7);
        assert_eq!(network::SET_RESPONSE_FUNCTIONS, 28);
        assert_eq!(sim::ICC_CARD_STATUS, 9);
        assert_eq!(modem::BASEBAND_VERSION, 2);
    }

    #[test]
    fn a_call_carries_the_interface_and_the_number() {
        let p = call(NETWORK, 7);
        let bytes = p.bytes();
        // The interface name is in there, in the UTF-16 the parcel uses.
        let text: String = bytes
            .windows(2)
            .filter_map(|w| {
                let u = u16::from_le_bytes([w[0], w[1]]);
                if (0x20..0x7f).contains(&u) {
                    Some(u as u8 as char)
                } else {
                    None
                }
            })
            .collect();
        assert!(text.contains("IRadioNetwork"), "{text}");
    }
}
