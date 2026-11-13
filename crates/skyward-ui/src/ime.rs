//! Chinese on the nine keys: the sound of a character, and the people whose
//! names are made of them.
//!
//! A recovery has no input method of its own and no room for a dictionary
//! of words. What it can keep is one line per common character, giving the
//! sound of it without tones. From that the nine keys of a phone keypad
//! (2 = abc, 3 = def, ... 9 = wxyz) turn a short string of digits into a
//! list of characters to choose from, and a name into the digits that find
//! it, so a person can be found by typing the sound of their name even
//! before any character is chosen.
//!
//! The table is kept as a plain text file rather than as literals in this
//! file: it holds thousands of characters, and the typeface test reads
//! every literal the pages write.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// The common characters, in the order they are offered, each with its
/// sound written without tones. The first part is what names are made of,
/// then the first and second levels of GB2312.
const HANYU: &str = include_str!("../resources/ime/hanyu.txt");

/// A key of the nine-key keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NineCap {
    /// One of the digits `0`..`9`. Only 2..9 spell a sound.
    Digit(char),
    /// Take the last digit of a sound away, or the last character typed.
    Backspace,
    /// Put a space in the field.
    Space,
    /// Choose the first character offered, or open the first person found.
    Enter,
    /// Go back to the Latin keyboard.
    Latin,
    /// Throw the sound being spelled away.
    Clear,
}

struct Table {
    /// Characters in the order they are offered, with the digits their
    /// sound spells on the nine keys and how common the character is.
    order: Vec<(char, String, usize)>,
    /// The sound of a character, for writing out a name.
    sound: HashMap<char, &'static str>,
    /// The characters by their sound, with how common each of them is.
    by_sound: Vec<(char, &'static str, usize)>,
}

static TABLE: OnceLock<Table> = OnceLock::new();

fn table() -> &'static Table {
    TABLE.get_or_init(|| {
        let mut order = Vec::new();
        let mut sound = HashMap::new();
        let mut by_sound = Vec::new();
        for line in HANYU.lines() {
            let mut it = line.split('\t');
            let Some(ch) = it.next().and_then(|c| c.chars().next()) else { continue };
            let Some(py) = it.next() else { continue };
            let _ = it.next();
            let common: usize = it.next().and_then(|r| r.parse().ok()).unwrap_or(usize::MAX);
            order.push((ch, t9(py), common));
            sound.insert(ch, py);
            by_sound.push((ch, py, common));
        }
        Table { order, sound, by_sound }
    })
}

/// Which of the nine keys a letter or digit belongs to; anything else has
/// no key of its own.
fn key_of(ch: char) -> Option<char> {
    Some(match ch.to_ascii_lowercase() {
        'a'..='c' => '2',
        'd'..='f' => '3',
        'g'..='i' => '4',
        'j'..='l' => '5',
        'm'..='o' => '6',
        'p'..='s' => '7',
        't'..='v' => '8',
        'w'..='z' => '9',
        '2'..='9' => ch,
        _ => return None,
    })
}

/// The digits `text` spells on the nine keys.
pub fn t9(text: &str) -> String {
    text.chars().filter_map(key_of).collect()
}

/// How a name answers a query: its sound in full, by the first letter of
/// each character, and both of those as the nine keys spell them.
#[derive(Default)]
pub struct Codes {
    pub full: String,
    pub initials: String,
    pub t9_full: String,
    pub t9_initials: String,
}

/// The sounds of a name, character by character. Latin letters and digits
/// in it are kept as they are, so a name of two scripts still answers.
pub fn codes(text: &str) -> Codes {
    let mut out = Codes::default();
    let t = table();
    for ch in text.chars() {
        if let Some(&py) = t.sound.get(&ch) {
            out.full.push_str(py);
            if let Some(first) = py.chars().next() {
                out.initials.push(first);
            }
        } else if ch.is_ascii_alphanumeric() {
            let ch = ch.to_ascii_lowercase();
            out.full.push(ch);
            out.initials.push(ch);
        }
    }
    out.t9_full = t9(&out.full);
    out.t9_initials = t9(&out.initials);
    out
}

/// Whether `name` answers `query`: as it is written, by its sound in full
/// or by initials, or by the digits those spell on the nine keys.
pub fn matches_name(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    if name.to_lowercase().contains(&q) {
        return true;
    }
    let c = codes(name);
    if q.chars().all(|ch| ch.is_ascii_lowercase()) {
        let typed = t9(&q);
        return c.full.contains(&q) || c.initials.contains(&q) || c.t9_full.contains(&typed) || c.t9_initials.contains(&typed);
    }
    if q.chars().all(|ch| ('2'..='9').contains(&ch)) {
        return c.t9_full.contains(&q) || c.t9_initials.contains(&q);
    }
    false
}

/// How many characters a page of choices holds.
pub const CHOICES: usize = 9;
/// How many characters are kept for one sound, over all of its pages.
const KEPT: usize = CHOICES * 10;

/// The characters whose sound begins with `digits`, best first: the ones
/// the phone's people are named with, then the shorter sounds, then the
/// order of the table. `favourite` holds the characters of their names.
pub fn candidates(digits: &str, favourite: &HashSet<char>) -> Vec<char> {
    if digits.is_empty() || !digits.chars().all(|ch| ('2'..='9').contains(&ch)) {
        return Vec::new();
    }
    let mut found: Vec<(u8, usize, usize, char)> = Vec::new();
    for (k, (ch, code, common)) in table().order.iter().enumerate() {
        if !code.starts_with(digits) {
            continue;
        }
        let named = u8::from(!favourite.contains(ch));
        found.push((named, *common, k, *ch));
    }
    found.sort_by_key(|&(named, common, k, _)| (named, common, k));
    found.into_iter().take(KEPT).map(|(_, _, _, ch)| ch).collect()
}

/// The characters the phone's people are named with, for [`candidates`] to
/// offer before the rest.
pub fn name_chars<'a>(names: impl IntoIterator<Item = &'a str>) -> HashSet<char> {
    names.into_iter().flat_map(str::chars).filter(|c| !c.is_ascii()).collect()
}

/// Every sound the table knows, with its nine-key code and the place of
/// its first character, which is how common it is.
static SYLLABLES: OnceLock<Vec<(String, String, usize)>> = OnceLock::new();

fn syllables() -> &'static Vec<(String, String, usize)> {
    SYLLABLES.get_or_init(|| {
        // A sound's rank is the commonest character that spells it, so
        // the sounds people write most come first.
        let mut rank: HashMap<&'static str, usize> = HashMap::new();
        for line in HANYU.lines() {
            let mut it = line.split('\t');
            let _ = it.next();
            let Some(py) = it.next() else { continue };
            let _ = it.next();
            let common: usize = it.next().and_then(|r| r.parse().ok()).unwrap_or(usize::MAX);
            match rank.get(py) {
                Some(&was) if was <= common => {}
                _ => {
                    rank.insert(py, common);
                }
            }
        }
        let mut all: Vec<(String, String, usize)> = rank
            .into_iter()
            .map(|(py, k)| (py.to_string(), t9(py), k))
            .collect();
        all.sort_by_key(|(_, _, k)| *k);
        all
    })
}

/// The sounds a word is made of, sound by sound: `张三` is `zhang` and
/// `san`, and `有信专线` is `you`, `xin`, `zhuan` and `xian`.
fn syllables_of(word: &str) -> Vec<String> {
    let full = codes(word).full;
    let syl = syllables();
    let mut out: Vec<String> = Vec::new();
    let mut at = 0;
    while at < full.len() {
        let mut hit: Option<&str> = None;
        for (py, _, _) in syl {
            if full[at..].starts_with(py) && hit.is_none_or(|h| py.len() > h.len()) {
                hit = Some(py);
            }
        }
        match hit {
            Some(py) => {
                out.push(py.to_string());
                at += py.len();
            }
            None => {
                out.push(full[at..at + 1].to_string());
                at += 1;
            }
        }
    }
    out
}

/// How many of the names this phone knows begin with these sounds: the
/// people are what a few keys are usually typed for, so the sound they
/// are called by is the one written out.
pub fn named_after(pinyin: &str, names: &[&str]) -> usize {
    let said: Vec<&str> = pinyin.split_whitespace().collect();
    if said.is_empty() {
        return 0;
    }
    names
        .iter()
        .filter(|n| !n.is_empty())
        .filter(|n| {
            let sounds = syllables_of(n);
            sounds.len() >= said.len() && sounds[..said.len()].iter().map(String::as_str).eq(said.iter().copied())
        })
        .count()
}

/// Every way the keys can spell a sound, the likeliest first. `94264` is
/// `zhang`, but it is `xiang` too, and a system keyboard offers both: the
/// sound written out, and the characters that sound spells.
pub fn spellings(digits: &str) -> Vec<String> {
    if digits.is_empty() || !digits.bytes().all(|b| (b'2'..=b'9').contains(&b)) {
        return Vec::new();
    }
    let syl = syllables();
    let mut found: Vec<(usize, usize, String)> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    walk_sounds(digits, 0, syl, &mut parts, 0, &mut found);
    found.sort_by_key(|(rank, count, _)| (*rank, *count));
    found.dedup_by(|a, b| a.2 == b.2);
    found.into_iter().take(8).map(|(_, _, said)| said).collect()
}

/// Every way of cutting the keys into sounds, sound by sound.
fn walk_sounds(
    digits: &str,
    at: usize,
    syl: &[(String, String, usize)],
    parts: &mut Vec<String>,
    rank: usize,
    out: &mut Vec<(usize, usize, String)>,
) {
    if parts.len() >= 4 || at >= digits.len() {
        return;
    }
    let rest = &digits[at..];
    let mut whole: Vec<&(String, String, usize)> = Vec::new();
    let mut other: Vec<&(String, String, usize)> = Vec::new();
    for item in syl.iter().filter(|(_, code, _)| rest.starts_with(code.as_str()) || code.starts_with(rest)) {
        if item.1.len() == rest.len() {
            whole.push(item);
        } else {
            other.push(item);
        }
    }
    // A sound the keys spell exactly is never left out: `94264` is
    // `zhang` and it is `xiang` too.
    whole.sort_by_key(|(_, _, r)| *r);
    other.sort_by_key(|(_, _, r)| *r);
    other.truncate(6);
    let here = whole.into_iter().chain(other);
    for (py, code, own) in here {
        if code.len() <= rest.len() {
            // The sound is typed out whole: what may follow it is looked
            // for as well.
            parts.push(py.clone());
            if code.len() == rest.len() {
                out.push((rank + own, parts.len(), parts.join(" ")));
            }
            walk_sounds(digits, at + code.len(), syl, parts, rank + own, out);
            parts.pop();
        } else {
            // The sound is only half typed: its letters are written out.
            let letters: String = py.chars().take(rest.len()).collect();
            parts.push(letters);
            out.push((rank + own, parts.len(), parts.join(" ")));
            parts.pop();
        }
    }
}

/// The keys a word spells, sound by sound: `张三` is `94264 726`.
fn sounds_of(word: &str) -> String {
    syllables_of(word).iter().map(|s| t9(s)).collect()
}

/// How common a character is written, and how common a word is: the
/// commonest character in it, with a word longer than two characters
/// counted against, for the fewer of them are meant.
fn common_of_char(ch: char) -> usize {
    table()
        .by_sound
        .iter()
        .find(|(c, _, _)| *c == ch)
        .map(|(_, _, k)| *k)
        .unwrap_or(usize::MAX)
}

fn common_of(text: &str) -> usize {
    text.chars().map(common_of_char).min().unwrap_or(usize::MAX)
}

/// The words an input method offers even when nobody is named with them:
/// what the recovery itself is called, so a feature can be looked for by
/// the sound of its name.
const EVERYDAY: &[&str] = &[
    "我们", "你们", "他们", "妈妈", "爸爸", "朋友", "同学", "老师", "公司", "家里", "手机", "电话", "中国", "今天", "明天", "昨天",
    "现在", "时间", "地址", "名字", "你好", "谢谢", "请问", "工作", "联系", "联系人", "通话记录",
    "备份", "恢复", "安装", "卸载", "挂载", "解密", "擦除", "清除数据", "双系统", "主题", "显示", "设置", "声音", "震动",
    "开发者", "网络", "无线网络", "蓝牙", "终端", "编辑器", "文件", "分区", "搜索", "升级", "更新", "下载", "密码", "锁屏", "屏幕",
    "键盘", "语言", "时区", "日期", "通话", "联系人列表", "回收站", "日志", "版本", "重启", "关机", "开机",
];

/// Everything a sound offers: the people this phone knows, the everyday
/// words, and then the characters themselves. Only what the sound spells
/// is offered, so the choice follows the sound that is written out; a
/// sound still being spelled offers every character that begins with it.
/// Each is given a score, so that the likeliest comes first: a character
/// by how common it is, a word by the commonest character in it, and the
/// names of this phone by how many of them there are.
pub fn choices_of(pinyin: &str, digits: &str, names: &[&str], favourite: &HashSet<char>, people_first: bool) -> Vec<(String, usize)> {
    let said: Vec<&str> = pinyin.split_whitespace().collect();
    if said.is_empty() {
        return Vec::new();
    }
    let code: String = said.iter().map(|s| t9(s)).collect();
    let named = named_after(pinyin, names);
    let _ = people_first;
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let offer = |word: &str, score: usize, out: &mut Vec<(String, usize)>, seen: &mut HashSet<String>| {
        if !word.is_empty() && seen.insert(word.to_string()) {
            out.push((word.to_string(), score));
        }
    };
    // The people the phone knows, whose names begin with this sound: `zhang`
    // offers 张 and the people called 张超 alike.
    let mut people: Vec<(usize, usize, &str)> = names
        .iter()
        .enumerate()
        .filter(|(_, name)| !name.is_empty() && sounds_of(name).starts_with(&code))
        .map(|(k, name)| (name.chars().count(), k, *name))
        .collect();
    people.sort_by_key(|&(len, k, _)| (len, k));
    for (_, _, name) in people.into_iter().take(14) {
        let len = name.chars().count();
        let score = common_of(name)
            .saturating_add(60 * len.saturating_sub(2))
            .saturating_sub(10 * named);
        offer(name, score, &mut out, &mut seen);
    }
    for word in EVERYDAY {
        if sounds_of(word).starts_with(&code) {
            let len = word.chars().count();
            let score = common_of(word).saturating_add(60 * len.saturating_sub(2));
            offer(word, score, &mut out, &mut seen);
        }
    }
    // The characters themselves, when the sound is a single one.
    if said.len() == 1 {
        let sound = said[0];
        let whole = syllables().iter().any(|(py, _, _)| py == sound);
        let mut chars: Vec<(usize, char)> = Vec::new();
        if whole {
            for (ch, py, common) in table().by_sound.iter() {
                if *py == sound {
                    chars.push((*common, *ch));
                }
            }
        } else {
            for (ch, cd, common) in table().order.iter() {
                if cd.starts_with(digits) {
                    chars.push((*common, *ch));
                }
            }
        }
        chars.sort_by_key(|(common, _)| *common);
        for (common, ch) in chars.into_iter().take(36) {
            let word = ch.to_string();
            let score = if favourite.contains(&ch) { common.saturating_sub(500) } else { common };
            offer(&word, score, &mut out, &mut seen);
        }
    }
    out
}

/// Everything the keys offer, over every sound they can spell: the likeliest
/// comes first, whichever sound it belongs to, as a system keyboard shows
/// them all and lets a finger choose.
pub fn choices(
    spellings: &[String],
    digits: &str,
    names: &[&str],
    favourite: &HashSet<char>,
    people_first: bool,
) -> Vec<(String, usize, usize)> {
    let mut out: Vec<(String, usize, usize)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (k, said) in spellings.iter().enumerate() {
        for (word, score) in choices_of(said, digits, names, favourite, people_first) {
            if seen.insert(word.clone()) {
                out.push((word, score, k));
            }
        }
    }
    out.sort_by_key(|(_, score, _)| *score);
    out
}


/// What a phone's pinyin bar shows for the keys pressed: the sound they
/// spell, or the letters of a sound still being spelled. The sounds the
/// same keys can spell are several; `spell_at` says which one is written
/// out, so a finger can turn it as a system keyboard does.
pub fn spelling(digits: &str, spell_at: usize) -> String {
    spellings(digits).get(spell_at).cloned().unwrap_or_default()
}

/// How common the characters a sound spells are: the first of them is the
/// one a finger is most likely to want.
pub fn commonness(said: &str) -> usize {
    let syls: Vec<&str> = said.split_whitespace().collect();
    if syls.len() != 1 {
        return usize::MAX;
    }
    let sound = syls[0];
    table()
        .by_sound
        .iter()
        .filter(|(_, py, _)| *py == sound)
        .map(|(_, _, common)| *common)
        .min()
        .unwrap_or(usize::MAX)
}

/// Whether every sound written out is a whole one: `zhang` is, and `zh`
/// is not, for that is only the beginning of a sound.
pub fn complete(said: &str) -> bool {
    let syls: Vec<&str> = said.split_whitespace().collect();
    !syls.is_empty()
        && syls
            .iter()
            .all(|s| syllables().iter().any(|(py, _, _)| py == *s))
}

/// Normalise a telephone number so the same number written with a country
/// code, spaces or dashes is still the same one: digits only, with a
/// leading `86` taken off a Chinese number long enough to have one.
pub fn number(text: &str) -> String {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if let Some(rest) = digits.strip_prefix("86") {
        if rest.len() >= 8 {
            return rest.to_string();
        }
    }
    digits
}

/// Whether two telephone numbers are the same one: either holds the other's
/// ending, as a call log without a country code meets a contact with one.
/// Seven digits is the shortest ending taken to be a number of its own.
pub fn same_number(a: &str, b: &str) -> bool {
    let (a, b) = (number(a), number(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a.len() < 7 || b.len() < 7 {
        return a == b;
    }
    let (long, short) = if a.len() >= b.len() { (&a, &b) } else { (&b, &a) };
    long.ends_with(short)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_spells_the_digits_of_its_letters() {
        assert_eq!(t9("zhang"), "94264");
        assert_eq!(t9("san"), "726");
        assert_eq!(t9("94264"), "94264");
    }

    #[test]
    fn a_name_answers_its_characters_and_its_sound() {
        assert!(matches_name("张三", "张"));
        assert!(matches_name("张三", "zhang"));
        assert!(matches_name("张三", "zs"));
        assert!(matches_name("张三", "94264"));
        assert!(matches_name("张三", "97"));
        assert!(!matches_name("张三", "lisi"));
    }

    #[test]
    fn the_nine_keys_offer_the_character_that_was_meant() {
        let none = HashSet::new();
        let zhang = candidates("94264", &none);
        assert!(zhang.contains(&'张'), "zhang offers 张: {zhang:?}");
        let san = candidates("726", &none);
        assert!(san.contains(&'三'), "san offers 三: {san:?}");
        assert!(candidates("", &none).is_empty());
    }

    #[test]
    fn a_character_in_a_persons_name_comes_first() {
        let named: HashSet<char> = "箱".chars().collect();
        let all = candidates("94264", &named);
        let plain = candidates("94264", &HashSet::new());
        assert!(all.contains(&'箱'), "the character is offered at all");
        assert_eq!(all.first(), Some(&'箱'), "it leads: {all:?}, plain {plain:?}");
    }

    #[test]
    fn the_keys_spell_a_sound_and_the_others_they_could_spell() {
        let all = spellings("94264");
        assert!(all.contains(&"zhang".to_string()), "zhang is one of them: {all:?}");
        assert!(all.contains(&"xiang".to_string()), "and so is xiang: {all:?}");
        assert_eq!(spelling("94264", 0), all[0]);
        assert_eq!(spelling("726", 0), "san", "the commonest sound of 726 comes first");
        assert_eq!(spelling("", 0), "");
    }

    #[test]
    fn a_name_says_which_sound_the_keys_were_typed_for() {
        let names = ["张三", "张超"];
        assert_eq!(named_after("zhang", &names), 2, "two names begin with zhang");
        assert_eq!(named_after("zhang san", &names), 1);
        assert_eq!(named_after("xiang", &names), 0, "but none with xiang");
    }

    #[test]
    fn a_sound_still_being_spelled_is_written_out_in_letters() {
        let half = spellings("94");
        assert!(half.iter().any(|s| s.starts_with("zh")), "{half:?}");
    }

    #[test]
    fn the_candidates_follow_the_sound_that_is_written_out() {
        let names = ["张三", "章伟"];
        let none = HashSet::new();
        let zhang = choices_of("zhang", "94264", &names, &none, true);
        assert!(zhang.iter().any(|(w, _)| w == "张"), "zhang offers 张: {zhang:?}");
        assert!(zhang.iter().any(|(w, _)| w == "张三"), "and the people: {zhang:?}");
        let xiang = choices_of("xiang", "94264", &names, &none, true);
        assert!(xiang.iter().any(|(w, _)| w == "相"), "xiang offers 相: {xiang:?}");
        assert!(!xiang.iter().any(|(w, _)| w == "张"), "but not 张: {xiang:?}");
    }

    #[test]
    fn every_sound_the_keys_can_spell_offers_its_characters() {
        // 96 is wo, and also ya and wa: the characters of all of them are
        // offered, as a system keyboard shows them.
        let none = HashSet::new();
        let all = spellings("96");
        assert!(all.iter().any(|s| s == "wo"), "wo is one of them: {all:?}");
        let got = choices(&all, "96", &[], &none, true);
        assert!(got.iter().any(|(w, _, _)| w == "我"), "我 is offered: {got:?}");
    }

    #[test]
    fn a_feature_can_be_looked_for_by_the_sound_of_its_name() {
        let none = HashSet::new();
        let got = choices_of("bei fen", "234336", &[], &none, false);
        assert!(got.iter().any(|(w, _)| w == "备份"), "{got:?}");
    }

    #[test]
    fn the_same_number_is_read_whatever_else_is_written_around_it() {
        assert!(same_number("+86 138 0013 8000", "13800138000"));
        assert!(same_number("138-0013-8000", "8613800138000"));
        assert!(!same_number("13800138000", "13900139000"));
        assert!(!same_number("", ""), "no number is nobody's number");
    }
}
