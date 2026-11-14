//! The browser's state, actions and effects, kept apart from the other pages.

use super::*;
use crate::icons::Icon;
use skyward_net::web::{FileOffer, Page as WebPage};

/// A shortcut on the browser's start page.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickLink {
    pub title: String,
    pub url: String,
    pub color: Rgba,
    pub icon: Icon,
}

/// The page the browser shows.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserView {
    Home,
    Loading(String),
    Page(WebPage),
    File(FileOffer),
    Full,
    Failed { url: String, why: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserSave {
    Downloading { done: u64, total: Option<u64> },
    Done,
    Stopped,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDownload {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size: Option<u64>,
    pub done: u64,
    pub state: BrowserSave,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub url: String,
    pub address: String,
    pub view: BrowserView,
    pub history: Vec<String>,
    pub at: usize,
    pub form: Vec<Vec<String>>,
    pub field: Option<(usize, usize)>,
}

impl BrowserTab {
    pub fn title(&self, _l: Lang) -> String {
        match &self.view {
            BrowserView::Page(page) if !page.title.is_empty() => page.title.clone(),
            BrowserView::File(file) => file.name.clone(),
            BrowserView::Loading(url) => url.clone(),
            BrowserView::Failed { url, .. } => url.clone(),
            _ => self.url.clone(),
        }
    }

    pub fn home() -> BrowserTab {
        BrowserTab {
            url: String::new(),
            address: String::new(),
            view: BrowserView::Home,
            history: Vec::new(),
            at: 0,
            form: Vec::new(),
            field: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserMark {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserFull {
    Unknown,
    Absent(String),
    Ready(String),
    Installing { done: u64, total: Option<u64> },
    Running(String),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginDownload {
    pub done: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Browser {
    pub address: String,
    pub url: String,
    pub view: BrowserView,
    pub history: Vec<String>,
    pub at: usize,
    pub downloads: Vec<BrowserDownload>,
    pub saving: Option<usize>,
    pub form: Vec<Vec<String>>,
    pub field: Option<(usize, usize)>,
    pub cookies: Vec<Cookie>,
    pub full: BrowserFull,
    pub full_shot: Option<Vec<u8>>,
    pub full_active: bool,
    pub full_zoom: f32,
    pub full_text: bool,
    pub full_touch: (u16, u16),
    pub full_progress: Option<f32>,
    pub tabs: Vec<BrowserTab>,
    pub active: usize,
    pub quick: Vec<QuickLink>,
    pub recent: Vec<BrowserMark>,
    pub bookmarks: Vec<BrowserMark>,
    pub plugin_download: Option<PluginDownload>,
}

impl Default for Browser {
    fn default() -> Self {
        Browser {
            address: String::new(),
            url: String::new(),
            view: BrowserView::Home,
            history: Vec::new(),
            at: 0,
            downloads: Vec::new(),
            saving: None,
            form: Vec::new(),
            field: None,
            cookies: Vec::new(),
            full: BrowserFull::Unknown,
            full_shot: None,
            full_active: false,
            full_zoom: 1.0,
            full_text: false,
            full_touch: (500, 500),
            full_progress: None,
            tabs: vec![BrowserTab::home()],
            active: 0,
            quick: Vec::new(),
            recent: Vec::new(),
            bookmarks: Vec::new(),
            plugin_download: None,
        }
    }
}

impl App {
    pub(super) fn browser_act(&mut self, action: Action, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        match action {
            Action::BrowserAddress => {
                self.state.browser.address = self.state.browser.url.clone();
                self.state.layer = Layer::Letters;
                self.open(Screen::BrowserAddress);
            }
            Action::BrowserBack => return self.browser_step(-1, now),
            Action::BrowserForward => return self.browser_step(1, now),
            Action::BrowserReload => {
                let url = self.state.browser.url.clone();
                if !url.is_empty() {
                    return self.browser_read(url);
                }
            }
            Action::BrowserHome => {
                self.state.browser.url.clear();
                self.state.browser.address.clear();
                self.state.browser.view = BrowserView::Home;
                self.state.browser.history.clear();
                self.state.browser.at = 0;
                self.open(Screen::Browser);
            }
            Action::BrowserTabs => self.open(Screen::BrowserTabs),
            Action::BrowserNewTab => {
                self.state.browser.new_tab();
                self.open(Screen::Browser);
            }
            Action::BrowserPickTab(i) => {
                self.state.browser.pick_tab(i);
                self.open(Screen::Browser);
            }
            Action::BrowserCloseTab(i) => {
                self.state.browser.close_tab(i);
                self.refresh();
            }
            Action::BrowserDownloads => self.open(Screen::BrowserDownloads),
            Action::BrowserLink(i) => {
                if let Some(url) = self.state.browser.page().and_then(|p| p.blocks.get(i)).and_then(|b| match b {
                    skyward_net::web::Block::Link { url, .. } => Some(url.clone()),
                    _ => None,
                }) {
                    return self.browser_read(url);
                }
            }
            Action::BrowserFormField(form, field) => {
                let b = &mut self.state.browser;
                if b.form.len() <= form {
                    b.form.resize(form + 1, Vec::new());
                }
                if b.form[form].len() <= field {
                    b.form[form].resize(field + 1, String::new());
                }
                b.field = Some((form, field));
                self.state.layer = Layer::Letters;
                self.open(Screen::BrowserForm);
            }
            Action::BrowserFormSubmit(form) => return self.browser_submit(form),
            Action::BrowserFormClose => {
                self.state.browser.field = None;
                self.back(now);
            }
            Action::BrowserStop => {
                let url = self.state.browser.url.clone();
                self.state.browser.view = BrowserView::Failed { url, why: l.pick(["已停止", "已停止", "Stopped"]).into() };
                self.refresh();
                return vec![Effect::StopBrowser];
            }
            Action::BrowserStopDownload => return vec![Effect::StopBrowser],
            Action::BrowserOpenDownload(i) => {
                let path = self.state.browser.downloads.get(i).map(|d| (d.path.clone(), matches!(d.state, BrowserSave::Done)));
                if let Some((path, true)) = path {
                    self.say(&path, now);
                }
            }
            Action::BrowserSaveFile => {
                if let Some(url) = self.state.browser.page().map(|p| p.url.clone()) {
                    let name = skyward_net::web::file_name(&url);
                    return self.browser_download(url, name, now);
                }
            }
            Action::BrowserClearAddress => {
                self.state.browser.address.clear();
                self.refresh();
            }
            Action::BrowserMenu => self.open(Screen::BrowserMenu),
            Action::BrowserHistoryList => self.open(Screen::BrowserHistory),
            Action::BrowserBookmarksList => self.open(Screen::BrowserBookmarks),
            Action::BrowserBookmark => {
                if let Some(page) = self.state.browser.page() {
                    let mark = BrowserMark { title: page.title.clone(), url: page.url.clone() };
                    if let Some(at) = self.state.browser.bookmarks.iter().position(|b| b.url == mark.url) {
                        self.state.browser.bookmarks.remove(at);
                    } else {
                        self.state.browser.bookmarks.push(mark);
                    }
                    self.refresh();
                }
            }
            Action::BrowserMarkOpen(i) => {
                let mark = match self.screen() {
                    Screen::BrowserHistory => self.state.browser.recent.get(i).cloned(),
                    Screen::BrowserBookmarks => self.state.browser.bookmarks.get(i).cloned(),
                    _ => None,
                };
                if let Some(mark) = mark {
                    self.open(Screen::Browser);
                    return self.browser_read(mark.url);
                }
            }
            Action::BrowserMarkRemove(i) => {
                let list = if self.screen() == Screen::BrowserHistory {
                    &mut self.state.browser.recent
                } else {
                    &mut self.state.browser.bookmarks
                };
                if i < list.len() {
                    list.remove(i);
                    self.refresh();
                }
            }
            Action::BrowserQuick => self.open(Screen::BrowserQuick),
            Action::BrowserQuickOpen(i) => {
                if let Some(q) = self.state.browser.quick.get(i).cloned() {
                    self.open(Screen::Browser);
                    return self.browser_read(q.url);
                }
            }
            Action::BrowserQuickAdd => {
                if let Some(page) = self.state.browser.page() {
                    if !self.state.browser.quick.iter().any(|q| q.url == page.url) {
                        self.state.browser.quick.push(QuickLink {
                            title: page.title.clone(),
                            url: page.url.clone(),
                            color: Rgba::rgb(0x34, 0x82, 0xff),
                            icon: Icon::Globe,
                        });
                        self.refresh();
                    }
                }
            }
            Action::BrowserQuickRemove(i) => {
                if i < self.state.browser.quick.len() {
                    self.state.browser.quick.remove(i);
                    self.refresh();
                }
            }
            Action::BrowserQuickReset => {
                self.state.browser.quick.clear();
                self.refresh();
            }
            Action::BrowserFull => {
                self.browser_check_full();
                self.open(Screen::BrowserFull);
            }
            Action::BrowserFullLoad => return self.browser_plugin_start_at(self.state.browser.url.clone()),
            Action::BrowserBasic => {
                self.state.browser.full_active = false;
                let url = self.state.browser.url.clone();
                self.open(Screen::Browser);
                if !url.is_empty() {
                    return self.browser_read(url);
                }
            }
            Action::BrowserPluginStart => return self.browser_plugin_start_at(self.state.browser.url.clone()),
            Action::BrowserPluginStop => {
                self.state.browser.full_active = false;
                return vec![Effect::BrowserPluginStop];
            }
            Action::BrowserPluginInstall => return self.browser_plugin_install(now),
            Action::BrowserPluginRemove => {
                let dir = "/data/media/0/Skyward/plugins/chromium".to_string();
                return vec![Effect::BrowserPluginRemove { dir }];
            }
            Action::BrowserPluginDownload => {
                self.state.browser.plugin_download = Some(PluginDownload { done: 0, total: None, speed: 0, error: None });
                return vec![Effect::BrowserPluginDownload {
                    url: "https://github.com/Jamvii/skyward-for-aurora/releases/latest/download/chromium.zip".into(),
                    path: "/sdcard/Download/Skyward/chromium.zip".into(),
                }];
            }
            Action::BrowserPluginReload => return vec![Effect::BrowserPluginReload],
            Action::BrowserPluginScroll { .. } => {}
            Action::BrowserZoomIn | Action::BrowserZoomOut => {}
            Action::BrowserScrollUp | Action::BrowserScrollDown => {}
            Action::BrowserKeyboard => {
                self.state.layer = Layer::Letters;
                self.state.browser.full_text = true;
                self.open(Screen::BrowserAddress);
            }
            Action::BrowserPluginTap(x, y) => return vec![Effect::BrowserPluginTap { x, y }],
            Action::BrowserPluginPinch(scale) => return vec![Effect::BrowserPluginPinch { scale: f64::from(scale) }],
            Action::BrowserPluginText => {}
            _ => {}
        }
        Vec::new()
    }

    pub(super) fn is_browser_action(action: Action) -> bool {
        is_browser_action(action)
    }

    pub(super) fn browser_go(&mut self, input: &str, _now: u64) -> Vec<Effect> {
        if self.state.browser.full_text {
            let text = input.trim().to_string();
            self.state.browser.full_text = false;
            self.state.browser.address.clear();
            if text.is_empty() {
                return Vec::new();
            }
            self.state.browser.full_progress = Some(0.2);
            self.refresh();
            return vec![Effect::BrowserPluginText(text)];
        }
        let url = skyward_net::web::address(input);
        if url.is_empty() {
            return Vec::new();
        }
        self.browser_read(url)
    }

    pub(super) fn browser_read(&mut self, url: String) -> Vec<Effect> {
        let b = &mut self.state.browser;
        if b.history.is_empty() {
            b.history.push(url.clone());
            b.at = 0;
        } else {
            b.history.truncate(b.at + 1);
            if b.history.last() != Some(&url) {
                b.history.push(url.clone());
                b.at = b.history.len() - 1;
            }
        }
        b.url = url.clone();
        b.address = url.clone();
        b.view = BrowserView::Loading(url.clone());
        b.form.clear();
        b.field = None;
        let cookie = cookie_header(&b.cookies, &url);
        self.refresh();
        vec![Effect::BrowserOpen { url, cookie }]
    }

    fn browser_cookie(&self, url: &str) -> Option<String> {
        cookie_header(&self.state.browser.cookies, url)
    }

    fn browser_step(&mut self, by: isize, _now: u64) -> Vec<Effect> {
        let b = &mut self.state.browser;
        let at = b.at as isize + by;
        if at < 0 || at as usize >= b.history.len() {
            return Vec::new();
        }
        b.at = at as usize;
        let url = b.history[b.at].clone();
        b.url = url.clone();
        b.address = url.clone();
        b.view = BrowserView::Loading(url.clone());
        let cookie = cookie_header(&b.cookies, &url);
        self.refresh();
        vec![Effect::BrowserOpen { url, cookie }]
    }

    fn browser_download(&mut self, url: String, name: String, _now: u64) -> Vec<Effect> {
        let path = format!("/sdcard/Download/Skyward/{name}");
        self.state.browser.downloads.push(BrowserDownload {
            name: name.clone(),
            url: url.clone(),
            path: path.clone(),
            size: None,
            done: 0,
            state: BrowserSave::Downloading { done: 0, total: None },
        });
        self.state.browser.saving = Some(self.state.browser.downloads.len() - 1);
        self.open(Screen::BrowserDownloads);
        self.refresh();
        vec![Effect::BrowserDownload { url, name, path }]
    }

    pub fn browser_opened(&mut self, opened: Result<skyward_net::web::Opened, String>) {
        let b = &mut self.state.browser;
        match opened {
            Ok(skyward_net::web::Opened::Page(page)) => {
                for cookie in &page.cookies {
                    b.cookies.retain(|kept| !(kept.name == cookie.name && kept.domain == cookie.domain && kept.path == cookie.path));
                    b.cookies.push(Cookie {
                        name: cookie.name.clone(),
                        value: cookie.value.clone(),
                        domain: cookie.domain.clone(),
                        path: cookie.path.clone(),
                        expires: None,
                    });
                }
                b.url = page.url.clone();
                b.address = page.url.clone();
                b.recent.retain(|m| m.url != page.url);
                b.recent.insert(0, BrowserMark { title: page.title.clone(), url: page.url.clone() });
                b.recent.truncate(60);
                b.view = BrowserView::Page(page);
            }
            Ok(skyward_net::web::Opened::File(file)) => {
                b.url = file.url.clone();
                b.address = file.url.clone();
                b.view = BrowserView::File(file);
            }
            Err(why) => {
                let url = b.url.clone();
                b.view = BrowserView::Failed { url, why };
            }
        }
        self.refresh();
    }

    pub fn browser_saved(&mut self, done: u64, total: Option<u64>) {
        let Some(i) = self.state.browser.saving else { return };
        let Some(d) = self.state.browser.downloads.get_mut(i) else { return };
        d.done = done;
        if total.is_some() {
            d.size = total;
        }
        d.state = BrowserSave::Downloading { done, total };
        self.refresh();
    }

    pub fn browser_save_ended(&mut self, ended: Result<(), String>, _now: u64) {
        let Some(i) = self.state.browser.saving.take() else { return };
        let Some(d) = self.state.browser.downloads.get_mut(i) else { return };
        d.state = match ended {
            Ok(()) => BrowserSave::Done,
            Err(why) => BrowserSave::Failed(why),
        };
        self.refresh();
    }

    fn browser_submit(&mut self, form: usize) -> Vec<Effect> {
        let Some(page) = self.state.browser.page() else { return Vec::new() };
        let Some(f) = page.forms.get(form) else { return Vec::new() };
        let mut fields = Vec::new();
        for (i, field) in f.fields.iter().enumerate() {
            let value = self.state.browser.form.get(form).and_then(|v| v.get(i)).cloned().unwrap_or_else(|| field.value.clone());
            if !field.name.is_empty() {
                fields.push((field.name.clone(), value));
            }
        }
        let url = f.action.clone();
        let post = f.post;
        self.state.browser.field = None;
        self.state.browser.view = BrowserView::Loading(url.clone());
        let cookie = self.browser_cookie(&url);
        self.refresh();
        vec![Effect::BrowserSubmit { url, fields, post, cookie }]
    }

    fn browser_check_full(&mut self) {
        let path = "/data/media/0/Skyward/plugins/chromium";
        self.state.browser.full = if std::path::Path::new(path).is_dir() {
            BrowserFull::Ready(path.into())
        } else {
            BrowserFull::Absent("插件未安装".into())
        };
    }

    fn browser_plugin_install(&mut self, _now: u64) -> Vec<Effect> {
        let dir = "/data/media/0/Skyward/plugins/chromium".to_string();
        for archive in [
            "/sdcard/Download/Skyward/chromium.zip",
            "/sdcard/Skyward/plugins/chromium.zip",
            "/sdcard/Skyward/plugins/chromium.tar.gz",
        ] {
            if std::path::Path::new(archive).is_file() {
                self.state.browser.full = BrowserFull::Installing { done: 0, total: None };
                return vec![Effect::BrowserPluginInstall { archive: archive.into(), dir }];
            }
        }
        Vec::new()
    }

    fn browser_plugin_start_at(&mut self, url: String) -> Vec<Effect> {
        let dir = "/data/media/0/Skyward/plugins/chromium".to_string();
        if !std::path::Path::new(&dir).is_dir() {
            self.state.browser.full = BrowserFull::Absent("插件未安装".into());
            self.open(Screen::BrowserFull);
            return Vec::new();
        }
        self.state.browser.full_active = true;
        self.state.browser.full = BrowserFull::Running("localhost:9222".into());
        self.state.browser.full_progress = Some(0.1);
        self.state.browser.view = BrowserView::Full;
        self.open(Screen::Browser);
        vec![Effect::BrowserPluginStart { dir, url, out: "/tmp/skyward-full.jpg".into() }]
    }

    pub fn browser_plugin_shot(&mut self, shot: Result<Vec<u8>, String>) {
        match shot {
            Ok(shot) => {
                self.state.browser.full_shot = Some(shot);
                self.state.browser.full_progress = Some(1.0);
                self.state.browser.view = BrowserView::Full;
            }
            Err(why) => {
                self.state.browser.full_active = false;
                self.state.browser.full_progress = None;
                let url = self.state.browser.url.clone();
                self.state.browser.view = BrowserView::Failed { url, why: why.clone() };
                self.state.browser.full = BrowserFull::Failed(why);
            }
        }
        self.refresh();
    }

    pub fn browser_plugin_focus(&mut self, focus: bool) {
        self.state.browser.full_text = focus;
        if focus {
            self.state.layer = Layer::Letters;
            if self.screen() == Screen::Browser {
                self.open(Screen::BrowserAddress);
            }
        }
    }

    pub fn browser_plugin_installed(&mut self, ended: Result<(), String>) {
        self.state.browser.full = match ended {
            Ok(()) => BrowserFull::Ready("/data/media/0/Skyward/plugins/chromium".into()),
            Err(why) => BrowserFull::Failed(why),
        };
        self.refresh();
    }

    pub fn plugin_saved(&mut self, done: u64, total: Option<u64>, speed: u64) {
        if let Some(d) = self.state.browser.plugin_download.as_mut() {
            d.done = done;
            d.total = total;
            d.speed = speed;
            d.error = None;
            self.refresh();
        }
    }

    pub fn plugin_downloaded(&mut self, ended: Result<(), String>) {
        if let Some(d) = self.state.browser.plugin_download.as_mut() {
            if let Err(why) = ended {
                d.error = Some(why);
            }
            self.refresh();
        }
    }
}

fn cookie_header(cookies: &[Cookie], url: &str) -> Option<String> {
    let pairs: Vec<(String, String, String, String)> = cookies
        .iter()
        .map(|c| (c.name.clone(), c.value.clone(), c.domain.clone(), c.path.clone()))
        .collect();
    skyward_net::web::cookie_header(&pairs, url)
}

fn is_browser_action(action: Action) -> bool {
    matches!(
        action,
        Action::BrowserAddress
            | Action::BrowserSaveFile
            | Action::BrowserQuickOpen(_)
            | Action::BrowserQuick
            | Action::BrowserQuickAdd
            | Action::BrowserQuickRemove(_)
            | Action::BrowserQuickReset
            | Action::BrowserHistory(_)
            | Action::BrowserDownloads
            | Action::BrowserFull
            | Action::BrowserBasic
            | Action::BrowserPluginTap(_, _)
            | Action::BrowserPluginPinch(_)
            | Action::BrowserPluginText
            | Action::BrowserPluginReload
            | Action::BrowserPluginStop
            | Action::BrowserPluginStart
            | Action::BrowserPluginInstall
            | Action::BrowserPluginRemove
            | Action::BrowserPluginDownload
            | Action::BrowserStop
            | Action::BrowserStopDownload
            | Action::BrowserOpenDownload(_)
            | Action::BrowserClearAddress
            | Action::BrowserTabs
            | Action::BrowserNewTab
            | Action::BrowserPickTab(_)
            | Action::BrowserCloseTab(_)
            | Action::BrowserMenu
            | Action::BrowserBookmark
            | Action::BrowserHistoryList
            | Action::BrowserBookmarksList
            | Action::BrowserMarkOpen(_)
            | Action::BrowserMarkRemove(_)
            | Action::BrowserLink(_)
            | Action::BrowserBack
            | Action::BrowserForward
            | Action::BrowserReload
            | Action::BrowserHome
            | Action::BrowserFormField(_, _)
            | Action::BrowserFormSubmit(_)
            | Action::BrowserFormClose
            | Action::BrowserFullLoad
            | Action::BrowserKeyboard
            | Action::BrowserScrollUp
            | Action::BrowserScrollDown
            | Action::BrowserZoomIn
            | Action::BrowserZoomOut
    )
}

/// What the browser's reader got.

impl Browser {
    pub fn page(&self) -> Option<&WebPage> {
        match &self.view {
            BrowserView::Page(page) => Some(page),
            _ => None,
        }
    }

    pub fn downloading(&self) -> bool {
        self.saving.is_some_and(|i| self.downloads.get(i).is_some_and(|d| matches!(d.state, BrowserSave::Downloading { .. })))
    }

    pub fn new_tab(&mut self) {
        self.tabs.push(BrowserTab::home());
        self.active = self.tabs.len() - 1;
        self.load_active();
    }

    pub fn close_tab(&mut self, i: usize) {
        if self.tabs.len() <= 1 || i >= self.tabs.len() {
            return;
        }
        self.tabs.remove(i);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        self.load_active();
    }

    pub fn pick_tab(&mut self, i: usize) {
        if i < self.tabs.len() {
            self.active = i;
            self.load_active();
        }
    }

    fn load_active(&mut self) {
        let tab = &self.tabs[self.active];
        self.url = tab.url.clone();
        self.address = tab.address.clone();
        self.view = tab.view.clone();
        self.history = tab.history.clone();
        self.at = tab.at;
        self.form = tab.form.clone();
        self.field = tab.field;
    }

    pub fn store_active(&mut self) {
        let tab = &mut self.tabs[self.active];
        tab.url = self.url.clone();
        tab.address = self.address.clone();
        tab.view = self.view.clone();
        tab.history = self.history.clone();
        tab.at = self.at;
        tab.form = self.form.clone();
        tab.field = self.field;
    }
}
