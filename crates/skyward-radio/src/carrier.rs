//! The phone's operator, as the top bar writes it.
//!
//! The radio says two things about the network it is on: a name, which the
//! Chinese networks spell in Chinese ("中国联通"), and a number, which is
//! the country and the network written as digits ("46001"). The name is
//! what a person reads, and the number is what tells the carriers apart
//! when the name is empty or in a language the reader does not know.
//!
//! So the number is what the carriers are told apart by here: a number of
//! five or six digits, of which the first three are the country and the
//! rest the network. China is 460, and each of its four networks has
//! numbers of its own.
//!
//! What the bars are called is a different matter: the radio counts the
//! signal on a scale of its own, and each kind of radio counts it on a
//! scale of its own again. What a person sees is four bars, so the scale
//! is turned into four bars here, in the way the phone's own framework
//! turns it into four bars.

/// The network the phone is on, as the four Chinese networks are told
/// apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carrier {
    /// 中国移动 — China Mobile.
    Mobile,
    /// 中国联通 — China Unicom.
    Unicom,
    /// 中国电信 — China Telecom.
    Telecom,
    /// 中国广电 — China Broadnet.
    Broadnet,
    /// A network of some other country, whose name is written as it
    /// comes.
    Other,
}

impl Carrier {
    /// The name of this network as the Chinese ones write it, or nothing
    /// for a network whose name the radio has to give.
    pub fn chinese(self) -> Option<&'static str> {
        match self {
            Carrier::Mobile => Some("中国移动"),
            Carrier::Unicom => Some("中国联通"),
            Carrier::Telecom => Some("中国电信"),
            Carrier::Broadnet => Some("中国广电"),
            Carrier::Other => None,
        }
    }

    /// The network a number of digits names, when the number is one China
    /// uses.
    ///
    /// The number is the country and the network written as digits, with
    /// no sign between them: "46001" is China, network 01. A number that
    /// is of another country is a network whose name is written as it
    /// comes.
    pub fn of_numeric(numeric: &str) -> Carrier {
        let digits: String = numeric.chars().filter(|c| c.is_ascii_digit()).collect();
        // China is 460; the two digits after it are the network.
        let Some(network) = digits.strip_prefix("460") else { return Carrier::Other };
        // A network of three digits is written with a leading zero kept,
        // so "460011" is network 011 and not 11.
        match network {
            "00" | "02" | "04" | "07" | "08" | "13" => Carrier::Mobile,
            "01" | "06" | "09" | "10" => Carrier::Unicom,
            "03" | "05" | "11" | "12" => Carrier::Telecom,
            "15" => Carrier::Broadnet,
            _ => Carrier::Other,
        }
    }

    /// The network a name names, when the name is one of the Chinese
    /// ones'.
    ///
    /// A radio that is asked on a network of another country gives that
    /// country's name in that country's own spelling, and nothing here
    /// knows it. A Chinese network gives its name in Chinese, and in
    /// English names too, so both are looked for.
    pub fn of_name(name: &str) -> Option<Carrier> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let folded = name.to_lowercase();
        let has = |words: [&str; 3]| words.iter().any(|w| folded.contains(&w.to_lowercase()));
        if has(["移动", "移动通", "Mobile"]) {
            Some(Carrier::Mobile)
        } else if has(["联通", "聯通", "Unicom"]) {
            Some(Carrier::Unicom)
        } else if has(["电信", "電信", "Telecom"]) {
            Some(Carrier::Telecom)
        } else if has(["广电", "廣電", "Broadnet"]) {
            Some(Carrier::Broadnet)
        } else {
            None
        }
    }

    /// The name to write in the top bar: the Chinese name of the
    /// network when it is one of the four, and otherwise the name the
    /// radio gave.
    ///
    /// The name the radio gave is preferred when it is already a Chinese
    /// name, since it is what the network calls itself; the number is
    /// only consulted when the name says nothing the reader can use.
    pub fn label(self, name: &str, numeric: &str) -> String {
        let name = name.trim();
        if !name.is_empty() {
            return name.to_string();
        }
        match Self::of_numeric(numeric) {
            Carrier::Other => name.to_string(),
            carrier => carrier.chinese().unwrap_or_default().to_string(),
        }
    }
}

/// How many bars a signal of the GSM count stands at.
///
/// The GSM count runs from 0 to 31, and the phone's own framework turns it
/// into four bars by doubling it, dividing by ten and taking one off,
/// which is what it does.
pub fn bars_of_gsm(level: i32) -> u8 {
    let bars = (level.max(0) * 2 / 10) - 1;
    bars.clamp(0, 4) as u8
}

/// How many bars a signal of the RSRP count stands at.
///
/// The RSRP count runs in dBm and the lower it is the weaker the signal;
/// the four bars are the four steps the phone's own framework uses.
pub fn bars_of_rsrp(rsrp: i32) -> u8 {
    match rsrp {
        d if d >= -95 => 4,
        d if d >= -105 => 3,
        d if d >= -115 => 2,
        d if d >= -125 => 1,
        _ => 0,
    }
}

/// How many bars a signal stands at, whatever radio it came from.
///
/// The technology is named as the radio names it: "gsm", "cdma", "evdo",
/// "lte", "tdscdma", "wcdma" or "nr". A signal of a kind this does not
/// know is shown as one bar, since a signal is there but its strength is
/// not known.
pub fn bars_of(technology: &str, level: i32, rsrp: i32) -> u8 {
    match technology.trim().to_ascii_lowercase().as_str() {
        "gsm" | "cdma" | "evdo" | "tdscdma" | "wcdma" => bars_of_gsm(level),
        "lte" | "nr" => bars_of_rsrp(rsrp),
        _ => u8::from(level > 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_chinese_networks_are_told_apart_by_their_numbers() {
        assert_eq!(Carrier::of_numeric("46000"), Carrier::Mobile);
        assert_eq!(Carrier::of_numeric("46002"), Carrier::Mobile);
        assert_eq!(Carrier::of_numeric("46007"), Carrier::Mobile);
        assert_eq!(Carrier::of_numeric("46001"), Carrier::Unicom);
        assert_eq!(Carrier::of_numeric("46006"), Carrier::Unicom);
        assert_eq!(Carrier::of_numeric("46009"), Carrier::Unicom);
        assert_eq!(Carrier::of_numeric("46003"), Carrier::Telecom);
        assert_eq!(Carrier::of_numeric("46005"), Carrier::Telecom);
        assert_eq!(Carrier::of_numeric("46011"), Carrier::Telecom);
        assert_eq!(Carrier::of_numeric("46015"), Carrier::Broadnet);
    }

    #[test]
    fn a_number_of_another_country_is_a_network_of_its_own() {
        assert_eq!(Carrier::of_numeric("310260"), Carrier::Other);
        assert_eq!(Carrier::of_numeric(""), Carrier::Other);
        assert_eq!(Carrier::of_numeric("46099"), Carrier::Other);
    }

    #[test]
    fn the_names_of_the_four_networks_are_recognised_in_both_languages() {
        assert_eq!(Carrier::of_name("中国联通"), Some(Carrier::Unicom));
        assert_eq!(Carrier::of_name("中国电信"), Some(Carrier::Telecom));
        assert_eq!(Carrier::of_name("中国移动"), Some(Carrier::Mobile));
        assert_eq!(Carrier::of_name("中国广电"), Some(Carrier::Broadnet));
        assert_eq!(Carrier::of_name("China Unicom"), Some(Carrier::Unicom));
        assert_eq!(Carrier::of_name("CHINA TELECOM"), Some(Carrier::Telecom));
        assert_eq!(Carrier::of_name("Orange"), None);
        assert_eq!(Carrier::of_name("  "), None);
    }

    #[test]
    fn the_name_the_radio_gives_is_what_is_written_when_it_says_something() {
        assert_eq!(Carrier::Unicom.label("中国联通", "46001"), "中国联通");
        // A network of another country keeps the name it came with.
        assert_eq!(Carrier::Other.label("Orange", "20801"), "Orange");
        // A name that says nothing falls back to the number's network.
        assert_eq!(Carrier::Unicom.label("", "46001"), "中国联通");
        assert_eq!(Carrier::Telecom.label("  ", "46003"), "中国电信");
    }

    #[test]
    fn the_gsm_count_is_turned_into_four_bars_the_way_the_phone_does() {
        assert_eq!(bars_of_gsm(0), 0);
        assert_eq!(bars_of_gsm(4), 0);
        assert_eq!(bars_of_gsm(5), 0);
        assert_eq!(bars_of_gsm(10), 1);
        assert_eq!(bars_of_gsm(15), 2);
        assert_eq!(bars_of_gsm(20), 3);
        assert_eq!(bars_of_gsm(25), 4);
        assert_eq!(bars_of_gsm(31), 4);
    }

    #[test]
    fn the_rsrp_count_is_turned_into_four_bars_the_way_the_phone_does() {
        assert_eq!(bars_of_rsrp(-140), 0);
        assert_eq!(bars_of_rsrp(-125), 1);
        assert_eq!(bars_of_rsrp(-115), 2);
        assert_eq!(bars_of_rsrp(-105), 3);
        assert_eq!(bars_of_rsrp(-95), 4);
        assert_eq!(bars_of_rsrp(-60), 4);
    }

    #[test]
    fn the_bars_come_from_the_count_of_the_radio_that_gave_them() {
        assert_eq!(bars_of("gsm", 20, 0), 3);
        assert_eq!(bars_of("lte", 0, -80), 4);
        assert_eq!(bars_of("nr", 0, -120), 1);
        // A kind of radio this does not know shows a signal, but not how
        // strong it is.
        assert_eq!(bars_of("something", 0, 0), 0);
        assert_eq!(bars_of("something", 12, 0), 1);
    }
}
