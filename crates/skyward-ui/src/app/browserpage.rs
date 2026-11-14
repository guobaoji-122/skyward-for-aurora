//! The browser and Magisk pages' state, and the methods the runtime
//! drives them with. Ported from jam's browser work.

use super::*;
use skyward_net::web::{FileOffer, Opened};
use crate::icons::Icon;
use skyward_net::web::Page as WebPage;


/// What the browser shows.

#[derive(Clone, Debug, PartialEq)]

pub enum BrowserView {
    /// The start page: the address, links to sites, what was read before,
    /// and the downloads.
    Home,
    /// A page being read, or a file being looked at, and its address.
    Loading(String),
    Page(WebPage),
    File(FileOffer),
    /// A page drawn by the external Chromium plugin, shown under the normal
    /// browser bar and dock.
    Full,
    /// The address asked for, and why nothing came back.
    Failed { url: String, why: String },
}

/// How a file the browser saves stands.

#[derive(Clone, Debug, PartialEq, Eq)]

pub enum BrowserSave {
    Downloading { done: u64, total: Option<u64> },
    Done,
    Stopped,
    Failed(String),
}

/// What the optional Linux Chromium plugin is doing.

#[derive(Clone, Debug, PartialEq, Eq)]

pub enum BrowserFull {
    /// Not looked for yet.
    Unknown,
    /// The plugin is not there: why.
    Absent(String),
    /// The plugin's files are there, at this folder.
    Ready(String),
    /// The plugin is being fetched or unpacked.
    Installing { done: u64, total: Option<u64> },
    /// The plugin is running at this local debugging address.
    Running(String),
    Failed(String),
}

/// Magisk, KernelSU or APatch, and the modules it runs.

#[derive(Clone, Debug, PartialEq, Eq)]

pub enum MagiskRoot {
    Unknown,
    Locked,
    Absent,
    Present(String),
}

/// One open page, kept while another tab shows.

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

/// One of the start page's shortcuts.

#[derive(Clone, Debug, PartialEq)]

pub struct QuickLink {
    pub title: String,
    pub url: String,
    pub color: Rgba,
    pub icon: Icon,
}

/// One cookie the browser sends back to a server.

#[derive(Clone, Debug, PartialEq, Eq)]

pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    /// Seconds since the epoch, or none for the session.
    pub expires: Option<i64>,
}

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

pub struct Browser {
    /// What is typed in the address field.
    pub address: String,
    /// The address of the page shown.
    pub url: String,
    pub view: BrowserView,
    /// The addresses read, oldest first, and which of them shows.
    pub history: Vec<String>,
    pub at: usize,
    pub downloads: Vec<BrowserDownload>,
    /// The download under way, by its place in `downloads`.
    pub saving: Option<usize>,
    /// The values typed into the page's form fields, by form and field.
    pub form: Vec<Vec<String>>,
    /// Which field the keyboard types into, where one is chosen.
    pub field: Option<(usize, usize)>,
    /// The cookie jar: name, value, domain, path, and when it expires.
    pub cookies: Vec<Cookie>,
    /// The plugin's state, and where it was found or why it is not there.
    pub full: BrowserFull,
    /// The last page the plugin rendered, as PNG.
    pub full_shot: Option<Vec<u8>>,
    /// Whether the page shown is the plugin's, and new addresses go to it.
    pub full_active: bool,
    /// The plugin page's CSS zoom, changed by the full-mode zoom buttons.
    pub full_zoom: f32,
    /// The keyboard is typing into the plugin page's focused field.
    pub full_text: bool,
    /// The last place tapped on the plugin page, in per-mille.
    pub full_touch: (u16, u16),
    /// The plugin's progress, from 0 to 1.
    pub full_progress: Option<f32>,
    /// The open pages, and which one shows.
    pub tabs: Vec<BrowserTab>,
    pub active: usize,
    /// The start page's shortcuts.
    pub quick: Vec<QuickLink>,
}

/// One cookie the browser sends back to a server.

#[derive(Clone, Debug, PartialEq, Eq)]



/// What the optional Linux Chromium plugin is doing.

#[derive(Clone, Debug, PartialEq, Eq)]



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
            quick: default_quick(),
        }
    }
}

fn default_quick() -> Vec<QuickLink> {
    vec![
        QuickLink { title: "GitHub".into(), url: "https://github.com".into(), color: Rgba::rgb(0x34, 0x82, 0xff), icon: Icon::Globe },
        QuickLink {
            title: "Skyward".into(),
            url: "https://github.com/kirklin/skyward".into(),
            color: Rgba::rgb(0x77, 0x67, 0xf9),
            icon: Icon::Layers,
        },
        QuickLink {
            title: "LineageOS".into(),
            url: "https://download.lineageos.org".into(),
            color: Rgba::rgb(0x1d, 0xcd, 0x3a),
            icon: Icon::Install,
        },
        QuickLink {
            title: "Bing".into(),
            url: "https://www.bing.com".into(),
            color: Rgba::rgb(0xff, 0x69, 0x00),
            icon: Icon::Globe,
        },
    ]
}

impl Browser {
    pub fn page(&self) -> Option<&WebPage> {
        match &self.view {
            BrowserView::Page(page) => Some(page),
            _ => None,
        }
    }

    pub fn downloading(&self) -> bool {
        self.saving.is_some_and(|i| matches!(self.downloads.get(i).map(|d| &d.state), Some(BrowserSave::Downloading { .. })))
    }

    fn sync_tab(&mut self) {
        if self.active >= self.tabs.len() {
            self.tabs.push(BrowserTab::home());
            self.active = self.tabs.len() - 1;
        }
        self.tabs[self.active] = BrowserTab {
            url: self.url.clone(),
            address: self.address.clone(),
            view: self.view.clone(),
            history: self.history.clone(),
            at: self.at,
            form: self.form.clone(),
            field: self.field,
        };
    }

    fn load_tab(&mut self) {
        let Some(tab) = self.tabs.get(self.active).cloned() else { return };
        self.url = tab.url;
        self.address = tab.address;
        self.view = tab.view;
        self.history = tab.history;
        self.at = tab.at;
        self.form = tab.form;
        self.field = tab.field;
    }

    pub fn new_tab(&mut self) {
        self.sync_tab();
        self.tabs.push(BrowserTab::home());
        self.active = self.tabs.len() - 1;
        self.load_tab();
    }

    pub fn switch_tab(&mut self, at: usize) {
        if at >= self.tabs.len() {
            return;
        }
        self.sync_tab();
        self.active = at;
        self.load_tab();
    }

    pub fn close_tab(&mut self, at: usize) {
        if at >= self.tabs.len() {
            return;
        }
        if self.active == at {
            self.sync_tab();
        }
        self.tabs.remove(at);
        if self.tabs.is_empty() {
            self.tabs.push(BrowserTab::home());
            self.active = 0;
        } else if self.active > at {
            self.active -= 1;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active == at {
            self.active = self.active.min(self.tabs.len() - 1);
        }
        self.load_tab();
    }
}

pub struct Magisk {
    pub root: MagiskRoot,
    /// The modules found, or why they could not be read.
    pub modules: Option<Result<Vec<MagiskModule>, String>>,
    /// The installed Magisk version, when Magisk is the root solution.
    pub version: Option<String>,
    /// The newest Magisk release found, if it has been checked.
    pub latest: Option<String>,
}

impl Default for Magisk {
    fn default() -> Self {
        Magisk { root: MagiskRoot::Unknown, modules: None, version: None, latest: None }
    }
}

impl Browser {
    pub fn page(&self) -> Option<&WebPage> {
        match &self.view {
            BrowserView::Page(page) => Some(page),
            _ => None,
        }
    }

    pub fn downloading(&self) -> bool {
        self.saving.is_some_and(|i| matches!(self.downloads.get(i).map(|d| &d.state), Some(BrowserSave::Downloading { .. })))
    }

    fn sync_tab(&mut self) {
        if self.active >= self.tabs.len() {
            self.tabs.push(BrowserTab::home());
            self.active = self.tabs.len() - 1;
        }
        self.tabs[self.active] = BrowserTab {
            url: self.url.clone(),
            address: self.address.clone(),
            view: self.view.clone(),
            history: self.history.clone(),
            at: self.at,
            form: self.form.clone(),
            field: self.field,
        };
    }

    fn load_tab(&mut self) {
        let Some(tab) = self.tabs.get(self.active).cloned() else { return };
        self.url = tab.url;
        self.address = tab.address;
        self.view = tab.view;
        self.history = tab.history;
        self.at = tab.at;
        self.form = tab.form;
        self.field = tab.field;
    }

    pub fn new_tab(&mut self) {
        self.sync_tab();
        self.tabs.push(BrowserTab::home());
        self.active = self.tabs.len() - 1;
        self.load_tab();
    }

    pub fn switch_tab(&mut self, at: usize) {
        if at >= self.tabs.len() {
            return;
        }
        self.sync_tab();
        self.active = at;
        self.load_tab();
    }

    pub fn close_tab(&mut self, at: usize) {
        if at >= self.tabs.len() {
            return;
        }
        if self.active == at {
            self.sync_tab();
        }
        self.tabs.remove(at);
        if self.tabs.is_empty() {
            self.tabs.push(BrowserTab::home());
            self.active = 0;
        } else if self.active > at {
            self.active -= 1;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active == at {
            self.active = self.active.min(self.tabs.len() - 1);
        }
        self.load_tab();
    }
}

/// Whether a folder is on the OTG drive, which the file manager reads with

/// the USB port as host.

fn on_otg(path: &str) -> bool {
    path == USB_DRIVE || path.starts_with(&format!("{USB_DRIVE}/"))
}

fn read_dir_effect(path: String) -> Effect {
    if on_otg(&path) {
        Effect::ReadUsbDir(path)
    } else {
        Effect::ReadDir(path)
    }
}

fn escape_text(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The address the plugin should load. The basic reader asks Bing for RSS;

/// Chromium should get the same search without `format=rss`, or it renders

/// an XML feed instead of the page.

fn chromium_url(url: &str) -> String {
    let mut url = url
        .replace("?format=rss&", "?")
        .replace("&format=rss&", "&")
        .replace("?format=rss", "?")
        .replace("&format=rss", "");
    if url.ends_with('?') {
        url.pop();
    }
    url
}

const SNUG: u32 = 3200;

    #[test]
    fn a_home_page_grown_by_an_update_notice_scrolls_down_to_its_last_row() {
        let f = fonts();
        let plain = info();
        let mut app = App::new(Screen::Home, W, SNUG, PPD);
        let fr = app.frame(&f, &plain);
        let reboot = fr.hits.iter().find(|h| h.action == Action::Open(Screen::Reboot)).expect("a reboot row").rect;
        assert!(reboot.y + reboot.h <= SNUG as f32, "without the notice the rows fit the screen");
        let i = DeviceInfo { snapshot: SnapshotState::Snapshotted, ..info() };
        let mut app = App::new(Screen::Home, W, SNUG, PPD);
        let fr = app.frame(&f, &i);
        let reboot = fr.hits.iter().find(|h| h.action == Action::Open(Screen::Reboot)).expect("a reboot row").rect;
        assert!(reboot.y + reboot.h > SNUG as f32, "the notice pushes the last row below the screen");
        assert!(reboot.y + reboot.h <= fr.canvas.h as f32, "the page is drawn down to it");
        app.touch(Touch::Down(700.0, 2800.0), &f, 0);
        app.touch(Touch::Move(700.0, 2000.0), &f, 0);
        app.touch(Touch::Move(700.0, 1200.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        for ms in (0..3000).step_by(8) {
            app.tick(ms);
        }
        let (_, y) = spot(&mut app, &f, &i, Action::Open(Screen::Reboot));
        assert!(y < SNUG as f32, "scrolled up into view");
        tap_at(&mut app, &f, &i, Action::Open(Screen::Reboot), 3000);
        assert_eq!(app.screen(), Screen::Reboot);
    }

impl App {
fn set_magisk(&mut self, root: MagiskRoot, modules: Result<Vec<MagiskModule>, String>) {
        let version = self.state.magisk.version.clone();
        let latest = self.state.magisk.latest.clone();
        self.state.magisk = Magisk { root, modules: Some(modules), version, latest };
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

fn set_magisk_version(&mut self, version: Option<String>) {
        self.state.magisk.version = version;
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

fn set_magisk_latest(&mut self, latest: Option<String>) {
        self.state.magisk.latest = latest;
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

fn install_downloaded(&mut self, path: String, size: u64, now: u64) -> Vec<Effect> {
        self.install_opened(path, size, now)
    }

fn browser_go(&mut self, input: &str, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
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
            self.say(l.pick(["请输入网址或搜索词", "請輸入網址或搜尋詞", "Enter an address or search for something"]), now);
            return Vec::new();
        }
        if !self.state.online() {
            self.say(l.pick(["需先连接 WLAN", "需先連接 Wi-Fi", "Connect to Wi-Fi first"]), now);
            self.open(Screen::Wlan);
            return self.arrive_at(Screen::Wlan);
        }
        self.browser_read(url)
    }

fn browser_read(&mut self, url: String) -> Vec<Effect> {
        let url = if self.state.browser.full_active { chromium_url(&url) } else { url };
        let full = {
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
            if b.full_active {
                let dir = match &b.full {
                    BrowserFull::Ready(dir) => Some(dir.clone()),
                    BrowserFull::Running(_) => Some("/data/media/0/Skyward/plugins/chromium".into()),
                    _ => None,
                };
                if let Some(dir) = dir {
                    b.full_shot = None;
                    b.full_progress = Some(0.12);
                    let out = format!("{dir}/tmp/skyward-chromium.png");
                    Some((dir, url.clone(), out))
                } else {
                    b.full_active = false;
                    None
                }
            } else {
                None
            }
        };
        if self.screen() == Screen::Browser {
            self.top_mut().1 = 0.0;
        }
        self.refresh();
        if let Some((dir, url, out)) = full {
            return vec![Effect::BrowserPluginStart { dir, url, out }];
        }
        let cookie = self.browser_cookie(&url);
        vec![Effect::BrowserOpen { url, cookie }]
    }

fn browser_step(&mut self, at: usize, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        if self.state.browser.history.is_empty() || at >= self.state.browser.history.len() {
            self.say(l.pick(["没有更多页面了", "沒有更多頁面了", "There's no page that way"]), now);
            return Vec::new();
        }
        let url = if self.state.browser.full_active { chromium_url(&self.state.browser.history[at]) } else { self.state.browser.history[at].clone() };
        let full = {
            let b = &mut self.state.browser;
            b.at = at;
            b.url = url.clone();
            b.address = url.clone();
            b.view = BrowserView::Loading(url.clone());
            b.form.clear();
            b.field = None;
            if b.full_active {
                let dir = match &b.full {
                    BrowserFull::Ready(dir) => Some(dir.clone()),
                    BrowserFull::Running(_) => Some("/data/media/0/Skyward/plugins/chromium".into()),
                    _ => None,
                };
                if let Some(dir) = dir {
                    b.full_shot = None;
                    b.full_progress = Some(0.12);
                    let out = format!("{dir}/tmp/skyward-chromium.png");
                    Some((dir, url.clone(), out))
                } else {
                    b.full_active = false;
                    None
                }
            } else {
                None
            }
        };
        if self.screen() == Screen::Browser {
            self.top_mut().1 = 0.0;
        }
        self.refresh();
        if let Some((dir, url, out)) = full {
            return vec![Effect::BrowserPluginStart { dir, url, out }];
        }
        let cookie = self.browser_cookie(&url);
        vec![Effect::BrowserOpen { url, cookie }]
    }

fn browser_download(&mut self, url: String, name: String, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        if self.state.browser.downloading() {
            self.say(l.pick(["已有文件正在下载", "已有檔案正在下載", "A file is being downloaded already"]), now);
            return Vec::new();
        }
        if !self.state.online() {
            self.say(l.pick(["需先连接 WLAN", "需先連接 Wi-Fi", "Connect to Wi-Fi first"]), now);
            self.open(Screen::Wlan);
            return self.arrive_at(Screen::Wlan);
        }
        let name = if name.trim().is_empty() { skyward_net::web::file_name(&url) } else { name };
        let path = match self.state.data.storage_open() {
            true => format!("/sdcard/{}/{}", skyward_net::web::FOLDER, name),
            false => format!("/tmp/skyward-browser/{name}"),
        };
        let b = &mut self.state.browser;
        b.downloads.push(BrowserDownload {
            name: name.clone(),
            url: url.clone(),
            path: path.clone(),
            size: None,
            done: 0,
            state: BrowserSave::Downloading { done: 0, total: None },
        });
        b.saving = Some(b.downloads.len() - 1);
        self.refresh();
        vec![Effect::BrowserDownload { url, name, path }]
    }

fn browser_opened(&mut self, opened: Result<Opened, String>) {
        {
            let b = &mut self.state.browser;
            match opened {
                Ok(Opened::Page(page)) => {
                    let url = page.url.clone();
                    if let Some(slot) = b.history.get_mut(b.at) {
                        *slot = url.clone();
                    }
                    b.url = url.clone();
                    b.address = url;
                    b.form = page
                        .forms
                        .iter()
                        .map(|form| form.fields.iter().map(|field| field.value.clone()).collect())
                        .collect();
                    for cookie in &page.cookies {
                        b.cookies.retain(|kept: &Cookie| !(kept.name == cookie.name && kept.domain == cookie.domain && kept.path == cookie.path));
                        b.cookies.push(Cookie {
                            name: cookie.name.clone(),
                            value: cookie.value.clone(),
                            domain: cookie.domain.clone(),
                            path: cookie.path.clone(),
                            expires: None,
                        });
                    }
                    b.view = BrowserView::Page(page);
                    b.field = None;
                }
                Ok(Opened::File(file)) => {
                    let url = file.url.clone();
                    if let Some(slot) = b.history.get_mut(b.at) {
                        *slot = url.clone();
                    }
                    b.url = url.clone();
                    b.address = url;
                    b.view = BrowserView::File(file);
                }
                Err(why) => {
                    let url = b.url.clone();
                    b.view = BrowserView::Failed { url, why };
                }
            }
        }
        if self.screen() == Screen::Browser {
            self.top_mut().1 = 0.0;
        }
        self.refresh();
    }

fn browser_saved(&mut self, done: u64, total: Option<u64>) {
        let Some(i) = self.state.browser.saving else { return };
        let Some(d) = self.state.browser.downloads.get_mut(i) else { return };
        d.done = done;
        if total.is_some() {
            d.size = total;
        }
        d.state = BrowserSave::Downloading { done, total };
        if self.screen() == Screen::BrowserDownloads || (self.screen() == Screen::Browser && matches!(self.state.browser.view, BrowserView::Home)) {
            self.refresh();
        }
    }

fn browser_save_ended(&mut self, outcome: Result<u64, String>, now: u64) {
        let Some(i) = self.state.browser.saving.take() else { return };
        let l = self.state.lang;
        if let Some(d) = self.state.browser.downloads.get_mut(i) {
            let path = d.path.clone();
            let said = match outcome {
                Ok(size) => {
                    d.done = size;
                    d.size = Some(size);
                    d.state = BrowserSave::Done;
                    tr!(l, "已保存到 {path}", "已儲存到 {path}", "Saved to {path}")
                }
                Err(why) if why.to_ascii_lowercase().contains("stopped") => {
                    d.state = BrowserSave::Stopped;
                    l.pick(["已停止下载", "已停止下載", "Download stopped"]).to_string()
                }
                Err(why) => {
                    d.state = BrowserSave::Failed(why.clone());
                    tr!(l, "下载失败：{why}", "下載失敗：{why}", "Couldn't download it: {why}")
                }
            };
            self.say(&said, now);
        }
        self.refresh();
    }

fn browser_open_download(&mut self, i: usize, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        let Some(d) = self.state.browser.downloads.get(i) else { return Vec::new() };
        let (name, path, size, state) = (d.name.clone(), d.path.clone(), d.done, d.state.clone());
        match state {
            BrowserSave::Downloading { .. } => {
                self.say(l.pick(["正在下载，请稍候", "正在下載，請稍候", "Downloading. Wait for it to finish"]), now);
                Vec::new()
            }
            BrowserSave::Done => {
                if skyward_net::web::installable(&name) {
                    self.install_opened(path, size, now)
                } else {
                    let said = tr!(l, "文件位于 {path}", "檔案位於 {path}", "The file is at {path}");
                    self.say(&said, now);
                    Vec::new()
                }
            }
            BrowserSave::Stopped | BrowserSave::Failed(_) => {
                self.say(l.pick(["文件尚未下载完成", "檔案尚未下載完成", "The file isn't downloaded yet"]), now);
                Vec::new()
            }
        }
    }

fn browser_submit(&mut self, form: usize, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        let Some(page) = self.state.browser.page() else { return Vec::new() };
        let Some(f) = page.forms.get(form) else { return Vec::new() };
        let mut fields = Vec::new();
        for (i, field) in f.fields.iter().enumerate() {
            if field.kind == skyward_net::web::FieldKind::Submit {
                continue;
            }
            let value = self
                .state
                .browser
                .form
                .get(form)
                .and_then(|v| v.get(i))
                .cloned()
                .unwrap_or_else(|| field.value.clone());
            if field.name.is_empty() {
                continue;
            }
            fields.push((field.name.clone(), value));
        }
        let url = f.action.clone();
        let post = f.post;
        let cookie = self.browser_cookie(&url);
        self.state.browser.view = BrowserView::Loading(url.clone());
        self.refresh();
        let _ = (l, now);
        vec![Effect::BrowserSubmit { url, fields, post, cookie }]
    }

fn browser_cookie(&self, url: &str) -> Option<String> {
        let pairs: Vec<(String, String, String, String)> = self
            .state
            .browser
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone(), c.domain.clone(), c.path.clone()))
            .collect();
        skyward_net::web::cookie_header(&pairs, url)
    }

fn browser_check_full(&mut self) {
        let path = "/data/media/0/Skyward/plugins/chromium";
        if !self.state.data.storage_open() {
            self.state.browser.full = BrowserFull::Absent(
                self.state.lang.pick(["需先解密 data 分区", "需先解密 data 分割區", "Decrypt the data partition first"]).to_string(),
            );
        } else if std::path::Path::new(path).join("run.sh").exists() {
            self.state.browser.full = BrowserFull::Ready(path.to_string());
        } else {
            self.state.browser.full = BrowserFull::Absent(
                self.state.lang.pick(["插件尚未安装", "外掛尚未安裝", "The plugin isn't installed"]).to_string(),
            );
        }
    }

fn browser_plugin_install(&mut self, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        if !self.state.data.storage_open() {
            self.say(l.pick(["请先解密 data 分区", "請先解密 data 分割區", "Decrypt the data partition first"]), now);
            return Vec::new();
        }
        let dir = "/data/media/0/Skyward/plugins/chromium";
        let archive = [
            "/sdcard/Skyward/plugins/chromium.tar.gz",
            "/sdcard/Download/Skyward/chromium.tar.gz",
            "/sdcard/Skyward/plugins/chromium.zip",
            "/sdcard/Download/Skyward/chromium.zip",
        ]
        .into_iter()
        .find(|path| std::path::Path::new(path).exists());
        let Some(archive) = archive else {
            let said = tr!(
                l,
                "没找到插件包，请先把它下载到 /sdcard/Skyward/plugins/chromium.tar.gz",
                "找不到外掛包，請先將它下載到 /sdcard/Skyward/plugins/chromium.tar.gz",
                "The plugin package isn't at /sdcard/Skyward/plugins/chromium.tar.gz"
            );
            self.say(&said, now);
            return Vec::new();
        };
        self.state.browser.full = BrowserFull::Installing { done: 0, total: None };
        self.state.browser.full_progress = Some(0.0);
        self.refresh();
        vec![Effect::BrowserPluginInstall { archive: archive.into(), dir: dir.into() }]
    }

fn browser_plugin_start(&mut self, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        if !self.state.online() {
            self.say(l.pick(["需先连接 WLAN", "需先連接 Wi-Fi", "Connect to Wi-Fi first"]), now);
            self.open(Screen::Wlan);
            return self.arrive_at(Screen::Wlan);
        }
        let url = if self.state.browser.url.is_empty() {
            self.say(l.pick(["请先打开一个网页", "請先開啟一個網頁", "Open a page first"]), now);
            self.open(Screen::BrowserAddress);
            return self.arrive_at(Screen::BrowserAddress);
        } else {
            self.state.browser.url.clone()
        };
        self.browser_plugin_start_at(url, now)
    }

fn browser_plugin_tap(&mut self, x: u16, y: u16, now: u64) -> Vec<Effect> {
        if self.state.browser.url.is_empty() || self.state.browser.full_shot.is_none() {
            return Vec::new();
        }
        self.state.browser.full_touch = (x, y);
        self.state.browser.full_progress = Some(0.2);
        self.refresh();
        vec![Effect::BrowserPluginTap { x, y }]
    }

fn browser_plugin_start_at(&mut self, url: String, now: u64) -> Vec<Effect> {
        let l = self.state.lang;
        if !self.state.online() {
            self.say(l.pick(["需先连接 WLAN", "需先連接 Wi-Fi", "Connect to Wi-Fi first"]), now);
            self.open(Screen::Wlan);
            return self.arrive_at(Screen::Wlan);
        }
        let url = chromium_url(&url);
        let dir = match &self.state.browser.full {
            BrowserFull::Ready(dir) => dir.clone(),
            BrowserFull::Running(_) => "/data/media/0/Skyward/plugins/chromium".into(),
            _ => {
                self.say(l.pick(["插件尚未安装", "外掛尚未安裝", "The plugin isn't installed"]), now);
                return Vec::new();
            }
        };
        self.state.browser.full = BrowserFull::Running("127.0.0.1:9222".into());
        let keep = self.state.browser.full_active && self.state.browser.full_shot.is_some();
        self.state.browser.full_active = true;
        if !keep {
            self.state.browser.full_shot = None;
            self.state.browser.view = BrowserView::Loading(url.clone());
        }
        self.state.browser.full_progress = Some(0.12);
        self.state.browser.url = url.clone();
        self.state.browser.address = url.clone();
        let out = format!("{dir}/tmp/skyward-chromium.png");
        if self.screen() != Screen::Browser {
            self.open(Screen::Browser);
        }
        self.refresh();
        vec![Effect::BrowserPluginStart { dir, url, out }]
    }

fn browser_plugin_stop(&mut self) -> Vec<Effect> {
        if let BrowserFull::Running(_) = self.state.browser.full {
            self.state.browser.full = BrowserFull::Absent("已停止".into());
            self.state.browser.full_progress = None;
            self.state.browser.full_active = false;
            self.refresh();
        }
        vec![Effect::BrowserPluginStop]
    }

fn browser_plugin_remove(&mut self, now: u64) -> Vec<Effect> {
        let _ = now;
        let dir = "/data/media/0/Skyward/plugins/chromium";
        self.state.browser.full = BrowserFull::Absent("插件尚未安装".into());
        self.state.browser.full_shot = None;
        self.state.browser.full_progress = None;
        self.refresh();
        vec![Effect::BrowserPluginRemove { dir: dir.into() }]
    }

fn browser_plugin_installed(&mut self, ended: Result<(), String>) {
        self.state.browser.full = match ended {
            Ok(()) => BrowserFull::Ready("/data/media/0/Skyward/plugins/chromium".into()),
            Err(why) => BrowserFull::Failed(why),
        };
        self.state.browser.full_progress = None;
        self.refresh();
    }

fn browser_plugin_focus(&mut self, focus: bool) {
        if focus
            && self.state.browser.full_active
            && !self.state.browser.full_text
            && self.screen() == Screen::Browser
            && !matches!(self.state.browser.view, BrowserView::Home)
        {
            self.state.browser.full_text = true;
            self.state.browser.address.clear();
            self.state.layer = Layer::Letters;
            self.open(Screen::BrowserAddress);
        }
    }

fn browser_plugin_shot(&mut self, shot: Result<Vec<u8>, String>) {        match shot {
            Ok(png) => {
                self.state.browser.full_shot = Some(png);
                self.state.browser.full_progress = Some(1.0);
                if self.state.browser.full_active {
                    self.state.browser.view = BrowserView::Full;
                    if self.screen() == Screen::BrowserAddress {
                        self.open(Screen::Browser);
                    }
                }
                if let BrowserFull::Running(at) = &self.state.browser.full {
                    self.state.browser.full = BrowserFull::Running(at.clone());
                }
            }
            Err(why) => {
                self.state.browser.full_progress = None;
                self.state.browser.full_active = false;
                let url = self.state.browser.url.clone();
                self.state.browser.view = BrowserView::Failed { url, why: why.clone() };
                self.state.browser.full = BrowserFull::Failed(why);
            }
        }
        self.refresh();
    }
}
