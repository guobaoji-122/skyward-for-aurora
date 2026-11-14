# Port to the 0.3.1 pages

The 0.3.1 merge split `app.rs` into `app/` modules. Features are being ported
back one at a time.

## Done

- File manager search
  - a Search control in the file manager's dock
  - matches the current folder's names by letters, pinyin, and pinyin initials
  - opens the original entry, so folders, text files and installable files keep
    their normal behaviour
- Magisk / KernelSU / APatch
  - reads `/data/adb/modules`, `/data/adb/ksu/modules` and
    `/data/adb/ap/modules`
  - lists modules, enables or disables one, and marks one for removal
  - the page is under Advanced

## Not ported yet

- the browser: address bar, basic HTML reader, tabs, downloads, cookies, forms,
  full mode, CDP frames, plugin download
- Magisk update download and module installation from a zip
- Browser log export

The browser is the largest port: its UI was built on the deleted `app.rs` and
its runtime on the 0.3.4 `skyward` binary. It is kept on `backup-pre-031`.
