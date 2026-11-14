//! Magisk, KernelSU and APatch modules as recovery sees them under
//! `/data/adb`.

use crate::app::{Action, Frame, Hit};
use crate::canvas::Canvas;
use crate::color::Rgba;
use crate::icons::Icon;
use crate::pages::{Band, Ctx};
use crate::theme::Theme;
use crate::widgets::{self, Row, Trail};

const GREEN: Rgba = Rgba::rgb(0x1d, 0xcd, 0x3a);
const BLUE: Rgba = Rgba::rgb(0x34, 0x82, 0xff);

fn hits(rects: &[crate::geom::Rect], actions: &[Action]) -> Vec<Hit> {
    rects.iter().zip(actions).map(|(&rect, &action)| Hit { rect, action }).collect()
}

fn page(cx: &Ctx, h: u32) -> Canvas {
    let w = cx.t.m.w;
    Canvas::band(w, h, 0..h, 0..w, cx.t.pal.bg)
}

fn fit(c: Canvas, t: &Theme, bottom: f32) -> Canvas {
    c.crop_height(((bottom + t.m.margin * 2.0).ceil() as u32).max(t.m.h))
}

pub fn screen(cx: &Ctx) -> Frame {
    let (t, f, s, l) = (cx.t, cx.f, cx.state, cx.state.lang);
    let m = &t.m;
    let count = s.magisk.modules.as_ref().and_then(|m| m.as_ref().ok()).map_or(0, |modules| modules.len());
    let need = (count as f32 * 2.0 + 10.0) * m.dp(72.0) + m.h as f32;
    let mut c = page(cx, need.ceil() as u32);
    let mut out = Vec::new();
    let top = m.content_top;
    let mut y = widgets::section_first(&mut c, t, f, top, "Magisk / KernelSU / APatch");
    y = match &s.magisk.root {
        MagiskRoot::Unknown => widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(["正在读取 root 方案", "正在讀取 root 方案", "Reading the root solution"]),
        ),
        MagiskRoot::Locked => widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick(["需先解密 data 分区", "需先解密 data 分割區", "Decrypt the data partition first"]),
        ),
        MagiskRoot::Absent => widgets::footnote(
            &mut c,
            t,
            f,
            y,
            l.pick([
                "未检测到 Magisk、KernelSU 或 APatch。",
                "未偵測到 Magisk、KernelSU 或 APatch。",
                "No Magisk, KernelSU or APatch was found.",
            ]),
        ),
        MagiskRoot::Present(name) => {
            let list = widgets::list(&mut c, t, f, y, &[Row::new(name).icon(GREEN, Icon::Shield).trail(Trail::None)]);
            out.extend(hits(&list.rows, &[Action::ReadMagisk]));
            list.bottom
        }
    };
    y = widgets::section(&mut c, t, f, y, l.pick(["模块", "模組", "Modules"]));
    match &s.magisk.modules {
        None => {
            y = widgets::footnote(&mut c, t, f, y, l.pick(["正在读取模块", "正在讀取模組", "Reading the modules"]));
        }
        Some(Err(why)) => {
            y = widgets::footnote(&mut c, t, f, y, why);
        }
        Some(Ok(modules)) if modules.is_empty() => {
            y = widgets::footnote(&mut c, t, f, y, l.pick(["没有模块", "沒有模組", "No modules"]));
        }
        Some(Ok(modules)) => {
            for (i, module) in modules.iter().enumerate() {
                let state = if module.removing {
                    l.pick(["重启后移除", "重新啟動後移除", "Removed after a restart"]).to_string()
                } else if module.enabled {
                    l.pick(["已启用", "已啟用", "Enabled"]).to_string()
                } else {
                    l.pick(["已禁用", "已停用", "Disabled"]).to_string()
                };
                let sub = format!("{} · {}", module.version, state);
                let row = Row::new(&module.name)
                    .subtitle(&sub)
                    .icon(if module.enabled { GREEN } else { BLUE }, Icon::Layers)
                    .trail(Trail::Switch(module.enabled && !module.removing));
                let list = widgets::list(&mut c, t, f, y, &[row]);
                out.extend(hits(&list.rows, &[Action::ToggleMagisk(i, !module.enabled)]));
                y = list.bottom;
                let row = Row::new(l.pick(["移除", "移除", "Remove"]))
                    .icon(Rgba::rgb(0xd1, 0x3a, 0x3a), Icon::Wipe)
                    .trail(Trail::None);
                let list = widgets::list(&mut c, t, f, y, &[row]);
                out.extend(hits(&list.rows, &[Action::RemoveMagisk(i)]));
                y = list.bottom;
            }
        }
    }
    Frame::new(t, fit(c, t, y), out, "Magisk", top)
}

/// Which root solution is installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MagiskRoot {
    Unknown,
    Locked,
    Absent,
    Present(String),
}

/// One module under `/data/adb`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MagiskModule {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub removing: bool,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Magisk {
    pub root: MagiskRoot,
    pub modules: Option<Result<Vec<MagiskModule>, String>>,
}

impl Default for Magisk {
    fn default() -> Self {
        Magisk { root: MagiskRoot::Unknown, modules: None }
    }
}
