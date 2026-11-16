//! The browser and Magisk pages' state, and the methods the runtime
//! drives them with. Ported from jam's browser work.

use super::*;
use skyward_net::web::Opened;
use crate::icons::Icon;
use skyward_net::web::Page as WebPage;




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

impl App {
pub fn set_magisk(&mut self, root: MagiskRoot, modules: Result<Vec<MagiskModule>, String>) {
        let version = self.state.magisk.version.clone();
        let latest = self.state.magisk.latest.clone();
        self.state.magisk = Magisk { root, modules: Some(modules), version, latest };
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

pub fn set_magisk_version(&mut self, version: Option<String>) {
        self.state.magisk.version = version;
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

pub fn set_magisk_latest(&mut self, latest: Option<String>) {
        self.state.magisk.latest = latest;
        if self.screen() == Screen::Magisk {
            self.refresh();
        }
    }

pub fn install_downloaded(&mut self, path: String, size: u64, now: u64) -> Vec<Effect> {
        self.install_opened(path, size, now)
    }

pub fn browser_go(&mut self, input: &str, now: u64) -> Vec<Effect> {
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

pub fn browser_read(&mut self, url: String) -> Vec<Effect> {
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

pub fn browser_step(&mut self, at: usize, now: u64) -> Vec<Effect> {
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

pub fn browser_download(&mut self, url: String, name: String, now: u64) -> Vec<Effect> {
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

pub fn browser_opened(&mut self, opened: Result<Opened, String>) {
        {
            let b = &mut self.state.browser;
            match opened {
                Ok(Opened::Page(page)) => {
                    let url = page.url.clone();
                    let title = page.title.clone();
                    if let Some(slot) = b.history.get_mut(b.at) {
                        *slot = url.clone();
                    }
                    remember(&mut b.recent, &title, &url);
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
                    remember(&mut b.recent, &file.name, &url);
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

pub fn browser_saved(&mut self, done: u64, total: Option<u64>) {
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

pub fn browser_save_ended(&mut self, outcome: Result<u64, String>, now: u64) {
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

pub fn browser_open_download(&mut self, i: usize, now: u64) -> Vec<Effect> {
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

pub fn browser_submit(&mut self, form: usize, now: u64) -> Vec<Effect> {
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

pub fn browser_cookie(&self, url: &str) -> Option<String> {
        let pairs: Vec<(String, String, String, String)> = self
            .state
            .browser
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone(), c.domain.clone(), c.path.clone()))
            .collect();
        skyward_net::web::cookie_header(&pairs, url)
    }

pub fn browser_check_full(&mut self) {
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

pub fn browser_plugin_install(&mut self, now: u64) -> Vec<Effect> {
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

pub fn browser_plugin_start(&mut self, now: u64) -> Vec<Effect> {
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

pub fn browser_plugin_tap(&mut self, x: u16, y: u16, now: u64) -> Vec<Effect> {
        if self.state.browser.url.is_empty() || self.state.browser.full_shot.is_none() {
            return Vec::new();
        }
        self.state.browser.full_touch = (x, y);
        self.state.browser.full_progress = Some(0.2);
        self.refresh();
        vec![Effect::BrowserPluginTap { x, y }]
    }

pub fn browser_plugin_start_at(&mut self, url: String, now: u64) -> Vec<Effect> {
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

pub fn browser_plugin_stop(&mut self) -> Vec<Effect> {
        if let BrowserFull::Running(_) = self.state.browser.full {
            self.state.browser.full = BrowserFull::Absent("已停止".into());
            self.state.browser.full_progress = None;
            self.state.browser.full_active = false;
            self.refresh();
        }
        vec![Effect::BrowserPluginStop]
    }

pub fn browser_plugin_remove(&mut self, now: u64) -> Vec<Effect> {
        let _ = now;
        let dir = "/data/media/0/Skyward/plugins/chromium";
        self.state.browser.full = BrowserFull::Absent("插件尚未安装".into());
        self.state.browser.full_shot = None;
        self.state.browser.full_progress = None;
        self.refresh();
        vec![Effect::BrowserPluginRemove { dir: dir.into() }]
    }

pub fn browser_plugin_installed(&mut self, ended: Result<(), String>) {
        self.state.browser.full = match ended {
            Ok(()) => BrowserFull::Ready("/data/media/0/Skyward/plugins/chromium".into()),
            Err(why) => BrowserFull::Failed(why),
        };
        self.state.browser.full_progress = None;
        self.refresh();
    }

pub fn browser_plugin_focus(&mut self, focus: bool) {
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

pub fn browser_plugin_shot(&mut self, shot: Result<Vec<u8>, String>) {        match shot {
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

    /// Whether a double tap should light a dark panel, so the runtime keeps
    /// the touch controller reporting while the panel is off.
    pub fn double_tap_wake(&self) -> bool {
        self.state.double_tap
    }

    /// How far the clock face follows a finger on a dark panel, if at all.
    pub fn aod_drag(&self) -> Option<f32> {
        self.aod_drag
    }

    /// The next package waiting in an install queue, if a queue is running
    /// and the install that just ended went through.
    pub fn queued_next(&mut self) -> Option<String> {
        if !self.state.queue_active {
            return None;
        }
        let through = match &self.state.install {
            Install::Finished { outcome, .. } => {
                matches!(outcome, Outcome::Installed { .. } | Outcome::Flashed { .. } | Outcome::Ran { .. })
            }
            _ => return None,
        };
        if !through {
            if let Some(name) = &self.state.package_name {
                self.state.queue_stopped = Some(name.clone());
            }
            self.state.queue_active = false;
            return None;
        }
        let Some(next) = self.state.queue.first().cloned() else {
            self.state.queue_active = false;
            return None;
        };
        self.state.queue.remove(0);
        self.state.queue_stopped = None;
        let (path, name) = (next.path, next.name);
        self.begin_install(Install::Checking { size: next.size, stage: None }, next.source, Some(name));
        Some(path)
    }

    /// How far the plugin package's download has come.
    pub fn plugin_saved(&mut self, done: u64, total: Option<u64>, speed: u64) {
        if let Some(d) = self.state.browser.plugin_download.as_mut() {
            d.done = done;
            d.total = total;
            d.speed = speed;
            d.error = None;
            if self.screen() == Screen::BrowserFull {
                self.refresh();
            }
        }
    }

    /// The plugin package finished, or why it did not.
    pub fn plugin_downloaded(&mut self, ended: Result<(), String>) {
        if let Some(d) = self.state.browser.plugin_download.as_mut() {
            if let Err(why) = ended {
                d.error = Some(why);
            }
        }
        if self.screen() == Screen::BrowserFull {
            self.refresh();
        }
    }
}

/// Keep a page in the browser's history or bookmarks, newest first: the
/// same page read again moves to the front rather than standing twice.
pub fn remember(list: &mut Vec<BrowserMark>, title: &str, url: &str) {
    if url.is_empty() {
        return;
    }
    list.retain(|mark| mark.url != url);
    list.insert(0, BrowserMark { title: title.to_string(), url: url.to_string() });
    list.truncate(60);
}
