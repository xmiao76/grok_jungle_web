Checked-in `wasm-pack --target web` output of `engine/`.

Rebuild after changing the crate:

```text
npm run build:wasm
```

`tools/build-wasm.mjs` deletes wasm-pack's `.gitignore` so these artifacts stay in git for Cloudflare Pages (no Rust on the builder).
