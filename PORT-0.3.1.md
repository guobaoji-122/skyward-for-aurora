# Browser port to 0.3.1

The browser is being rebuilt on the 0.3.1 `app/` modules rather than copied from
the deleted `app.rs`.

## In this build

- Browser entry under Advanced
- Basic reader: address bar, typed addresses and Bing search, HTML text, links,
  pictures, forms and the browser's dock
- Tabs, downloads page, start page shortcuts, menu, history and bookmarks
- Full mode: plugin install/start/stop, Chromium frame and touch/keyboard/
  scroll controls
- Plugin package download

## Still rough

- full mode takes a fresh frame per action rather than a continuous stream
- file manager search is still in the dock, not yet a top search box
- OTG devices are not yet browsable from the file manager
- Magisk online update and zip installation are not ported
