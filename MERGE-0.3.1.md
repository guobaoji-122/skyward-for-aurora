# Skyward 0.3.1 merge

The `browser-v034` branch was merged with upstream `kirklin/skyward` tag `v0.3.1`
(`bd34c05`). Conflicts were resolved in upstream's favour, as requested.

## Result

- Upstream workspace version: `0.3.1`
- `devices/aurora.toml` remains in the tree
- `cargo build --release -p skyward --target aarch64-linux-android` succeeds
- `cargo test -p skyward-ui --lib`: 336 passed

## Important

Upstream 0.3.1 deleted `crates/skyward-ui/src/app.rs` and split the page state
machine into `crates/skyward-ui/src/app/`. The Aurora browser, Magisk manager,
CDP session and related UI were built on the old `app.rs` API and do not compile
against the new modules. They are therefore **not part of the merge**.

The pre-merge tree is kept on the `backup-pre-031` branch, pointing at
`33cdb79`. The browser features can be recovered from there, but porting them to
the 0.3.1 page modules is a separate project.

## Image

`work/skyward-v0.3.1-aurora-upstream.img`

SHA-256: `3df136451b2eb6bf77acba7ab0eff456500c194fea3c85dfc48a44d87c107bea`

This image is upstream 0.3.1 plus the Aurora device description. It does not
contain the browser or Magisk pages.
