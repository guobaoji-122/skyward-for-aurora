//! The aurora pages and the state that came with them: the AVB checks,
//! the bootloader message, the always-on display, the LocalSend endpoint,
//! and the settings the recovery keeps between its starts.

use super::*;
use serde::{Deserialize, Serialize};
use skyward_net::localsend::Status as LocalSendStatus;
use skyward_vab::avb::{Change as AvbChange, Reading as AvbReading};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AodStyle {
    /// `HH:MM` large, centered.
    #[default]
    Digits,
    /// The hour and the minute set askew, each as large as the screen
    /// allows.
    Offset,
    /// A clock face.
    Dial,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Starter {
    /// The command the bootloader acts on, such as `boot-recovery`. Empty
    /// where nothing is pending.
    pub command: String,
    /// The one piece of work that command and its arguments add up to.
    pub asks: Asking,
    /// Why the system asked, in its own words, such as `fs_mgr_mount_all`.
    pub reason: Option<String>,
    /// The arguments as they were written, for reading in full.
    pub args: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Queued {
    /// What the page calls it: the file's own name.
    pub name: String,
    /// Where it is.
    pub path: String,
    /// How big it is, which the page says before it is opened.
    pub size: u64,
    /// Where it came from, which the install page says beside it.
    pub source: Source,
}
const WAKE_SWIPE_DP: f32 = 64.0;

/// Where the AOD sits, in dp from its usual place, one each minute in
/// turn, so no pixel holds the light long enough to burn into the panel.
const AOD_SPOTS: [(f32, f32); 4] = [(0.0, 0.0), (5.0, -3.0), (0.0, 6.0), (-5.0, -3.0)];

/// The AOD's inks on the black panel: the clock the brightest, the day
/// and the charge left dimmer, the hint of how to leave it faintest.
const AOD_INK: Rgba = Rgba::rgb(0xC0, 0xC0, 0xC0);
const AOD_DIM: Rgba = Rgba::rgb(0x82, 0x82, 0x82);
const AOD_FAINT: Rgba = Rgba::rgb(0x52, 0x52, 0x52);


impl Starter {
    /// Whether misc holds anything for the bootloader to act on.
    pub fn pending(&self) -> bool {
        !self.command.is_empty()
    }
}

impl AodStyle {
    /// The name the style is known by, as the setting shows it.
    pub fn name(self, l: Lang) -> &'static str {
        match self {
            AodStyle::Digits => l.pick(["数字", "數字", "Digits"]),
            AodStyle::Offset => l.pick(["错位", "錯位", "Offset"]),
            AodStyle::Dial => l.pick(["表盘", "錶盤", "Dial"]),
        }
    }

    /// The style after this one, as the setting is tapped.
    pub fn next(self) -> Self {
        match self {
            AodStyle::Digits => AodStyle::Offset,
            AodStyle::Offset => AodStyle::Dial,
            AodStyle::Dial => AodStyle::Digits,
        }
    }
}

impl App {
    pub fn queue_dropped(&mut self, i: usize) {
        if i < self.state.queue.len() {
            self.state.queue.remove(i);
            if self.state.queue.is_empty() {
                self.state.queue_stopped = None;
            }
        }
        self.refresh();
    }


    /// How the LocalSend endpoint stands, as the runtime last told it.
    pub fn set_local_send(&mut self, s: LocalSendStatus) {
        if self.state.local_send == s {
            return;
        }
        self.state.local_send = s;
        if matches!(self.screen(), Screen::Wlan | Screen::LocalSend) {
            self.refresh();
        }
    }

    /// What the phone's vbmeta blocks say, as they were just read, and what
    /// the bootloader made of them when the phone started.
    pub fn set_avb(&mut self, read: Result<Vec<AvbReading>, String>, verity: Option<String>) {
        self.state.avb = Some(read);
        self.state.verity = verity;
        self.refresh();
    }

    /// How turning the two checks the vbmeta blocks name off or on went, with
    /// what the blocks then said.
    pub fn avb_written(&mut self, outcome: Result<Vec<AvbChange>, String>, readings: Result<Vec<AvbReading>, String>, now: u64) {
        match &outcome {
            Ok(changes) => {
                let changed = changes.iter().filter(|c| c.kept.is_some()).count();
                if changed == 0 {
                    self.say(self.state.lang.pick(["已是这个状态，没有改动", "已是這個狀態，沒有改動", "Already as asked, nothing changed"]), now);
                } else {
                    let said = tr!(
                        self.state.lang,
                        "已改动 {changed} 个 vbmeta 区块，原样已备份",
                        "已改動 {changed} 個 vbmeta 區塊，原樣已備份",
                        "{changed} vbmeta blocks changed, the older ones kept"
                    );
                    self.say(&said, now);
                }
            }
            Err(why) => {
                // The operation stopped before finishing. Some blocks may
                // already have been written (e.g. when disabling and a
                // later block fails to stash or store), or none may have
                // been touched yet. The re-read `readings` below shows the
                // actual current state, so we describe the result as
                // incomplete rather than claiming nothing changed.
                let said = tr!(self.state.lang, "操作未完成：{why}", "操作未完成：{why}", "Operation incomplete: {why}");
                self.say(&said, now);
            }
        }
        self.state.avb_result = Some(outcome);
        self.state.avb = Some(readings);
        self.refresh();
    }

    /// What the bootloader message in misc asks for, as it was just read.
    pub fn set_starter(&mut self, read: Result<Starter, String>) {
        self.state.starter = Some(read);
        self.refresh();
    }

    /// How taking the bootloader message away went, with what misc holds
    /// then.
    pub fn starter_cleared(&mut self, outcome: Result<(), String>, readings: Result<Starter, String>, now: u64) {
        match &outcome {
            Ok(()) => self.say(self.state.lang.pick(["启动指令已清除", "啟動指令已清除", "The bootloader message is gone"]), now),
            Err(why) => {
                let said = tr!(self.state.lang, "没有清除：{why}", "沒有清除：{why}", "Nothing was cleared: {why}");
                self.say(&said, now);
            }
        }
        self.state.starter_cleared = Some(outcome);
        self.state.starter = Some(readings);
        self.refresh();
    }

    /// How switching the slot to start next went: what the boot control

    /// `drag` is how far a finger has swiped up on the dark glass, in dp:
    /// the face goes up with the finger and fades as it goes, so that the
    /// panel comes up where the finger has brought it and not all at once.
    /// None is a finger that is not on the glass.
    pub fn compose_aod(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, drag: Option<f32>) {
        out.fill_rect(0.0, 0.0, out.w as f32, out.h as f32, Rgba::rgb(0, 0, 0));
        let t = self.theme();
        let m = t.m;
        let l = self.state.lang;
        let (w, h) = (out.w as f32, out.h as f32);
        let minute = clock.get(3..5).and_then(|m| m.parse::<u32>().ok()).unwrap_or(0);
        let (dx, dy) = AOD_SPOTS[(minute as usize) % AOD_SPOTS.len()];
        let (dx, dy) = (m.dp(dx), m.dp(dy));
        let cx = w / 2.0 + dx;
        let date = self.state.aod_date.clone();
        let (hh, mm) = clock.split_once(':').unwrap_or(("00", "00"));
        match self.state.aod_style {
            AodStyle::Digits => {
                let cy = h * 0.34 + dy;
                out.text_center(&f.body, &date, cx, cy - m.dp(64.0), m.dp(16.0), AOD_DIM);
                out.text_center(&f.title, clock, cx, cy, m.dp(88.0), AOD_INK);
            }
            AodStyle::Offset => {
                out.text_center(&f.title, hh, cx - m.dp(90.0), h * 0.28 + dy, m.dp(150.0), AOD_INK);
                out.text_center(&f.title, mm, cx + m.dp(90.0), h * 0.28 + dy + m.dp(150.0), m.dp(150.0), AOD_INK);
                out.text_center(&f.body, &date, cx, h * 0.28 + dy + m.dp(252.0), m.dp(16.0), AOD_DIM);
            }
            AodStyle::Dial => {
                let cy = h * 0.40 + dy;
                self.aod_dial(out, f, (cx, cy, m.dp(104.0)), clock, info.battery.as_ref().map(|b| b.level));
                out.text_center(&f.body, &date, cx, cy - m.dp(104.0) - m.dp(46.0), m.dp(16.0), AOD_DIM);
            }
        }
        if let Some(battery) = &info.battery {
            let level = battery.level;
            let words = if battery.plugged {
                tr!(l, "充电中 {level}%", "充電中 {level}%", "Charging {level}%")
            } else {
                format!("{level}%")
            };
            out.text_center(&f.body, &words, cx, h * 0.66 + dy, m.dp(15.0), AOD_DIM);
        }
        let hint = l.pick(["上滑解锁", "上滑解鎖", "Swipe up to unlock"]);
        out.text_center(&f.body, hint, cx, h * 0.85 + dy, m.dp(13.0), AOD_FAINT);
        // A finger on the glass takes the face up with it, and the face
        // fades as it goes: it is leaving, and the page is coming.
        if let Some(drag) = drag {
            // The drag is in pixels; the threshold it is measured against
            // is in dp, as the gesture itself is.
            let gone = (drag / m.dp(WAKE_SWIPE_DP)).clamp(0.0, 1.0);
            out.lift(-gone * h * 0.35, 1.0 - gone * 0.85);
        }
    }

    /// The AOD's clock face: a ring with a subtle face, twelve marks, the
    /// quarter hours named inside them, tapered hour and minute hands at
    /// the time `(hh, mm)` name, and the charge left as an arc of the
    /// battery's share just outside the ring.
    fn aod_dial(&self, out: &mut Canvas, f: &Fonts, (cx, cy, r): (f32, f32, f32), clock: &str, level: Option<u8>) {
        let (hh, mm) = match clock.split_once(':').and_then(|(hh, mm)| hh.parse::<f32>().ok().zip(mm.parse::<f32>().ok())) {
            Some((hh, mm)) => (hh, mm),
            None => return,
        };
        let t = self.theme();
        let m = t.m;
        out.fill_circle(cx, cy, r, AOD_DIM);
        out.fill_circle(cx, cy, r - m.dp(3.0), Rgba::rgb(0x0A, 0x0A, 0x0A));
        if let Some(level) = level.filter(|l| *l > 0) {
            let turn = std::f32::consts::TAU;
            out.stroke_arc(cx, cy, r + m.dp(10.0), -std::f32::consts::FRAC_PI_2, turn * f32::from(level) / 100.0, m.dp(4.0), AOD_INK);
        }
        // Twelve marks, longer and thicker at the quarters, and the quarter
        // hours named inside them.
        for i in 0..12 {
            let a = i as f32 * std::f32::consts::FRAC_PI_6;
            let (sx, sy) = (a.sin(), -a.cos());
            let (px, py) = (-sy, sx);
            let quarter = i % 3 == 0;
            let (r0, r1, half) = if quarter {
                (r - m.dp(14.0), r - m.dp(6.0), m.dp(2.5))
            } else {
                (r - m.dp(10.0), r - m.dp(6.0), m.dp(1.25))
            };
            out.fill_convex(
                &[(cx + sx * r0 + px * half, cy + sy * r0 + py * half), (cx + sx * r1 + px * half, cy + sy * r1 + py * half), (cx + sx * r1 - px * half, cy + sy * r1 - py * half), (cx + sx * r0 - px * half, cy + sy * r0 - py * half)],
                AOD_INK,
            );
            if quarter {
                let n = if i == 0 { "12" } else { &i.to_string()[..] };
                out.text_center(&f.heading, n, cx + sx * r * 0.72, cy + sy * r * 0.72, m.dp(26.0), AOD_INK);
            }
        }
        // A hand, tapered from its base at the center out to its tip, at
        // `angle` from twelve, clockwise.
        let mut hand = |angle: f32, len: f32, base: f32, tip: f32| {
            let (sx, sy) = (angle.sin(), -angle.cos());
            let (px, py) = (-sy, sx);
            out.fill_convex(
                &[(cx - px * base, cy - py * base), (cx + sx * len + px * tip, cy + sy * len + py * tip), (cx + sx * len - px * tip, cy + sy * len - py * tip), (cx + px * base, cy + py * base)],
                AOD_INK,
            );
        };
        let turn = std::f32::consts::TAU;
        hand(hh.rem_euclid(12.0) / 12.0 * turn + mm / 60.0 * (turn / 12.0), r * 0.48, m.dp(5.0), m.dp(1.5));
        hand(mm / 60.0 * turn, r * 0.76, m.dp(3.5), m.dp(1.0));
        out.fill_circle(cx, cy, m.dp(6.0), AOD_INK);
        out.fill_circle(cx, cy, m.dp(2.0), Rgba::rgb(0x0A, 0x0A, 0x0A));
}
}
