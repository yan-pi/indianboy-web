# indianboy-web

A Rust-built personal blog and portfolio for [indianboy.sh](https://indianboy.sh).

The site builder compiles Markdown+ content into static HTML, feeds, metadata, and assets. A small `workers-rs` WebAssembly Worker runs on Cloudflare Workers and delegates generated assets to Cloudflare Static Assets.

## Local build

```sh
cargo run -p site-builder
```

The generated site is written to `dist/`. All 13 current posts are migrated into `content/posts/`; the remaining parity work is tracked in `docs/PARITY.md`.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm ci
npx playwright install chromium
```

With a local Worker running, execute the HTTP and browser smoke tests:

```sh
./tests/route-smoke.sh
BASE_URL=http://localhost:8787 npm run test:browser
```

## Cloudflare preview

Install `worker-build` and Wrangler, then run:

```sh
wrangler dev
```

## Design

The visual direction is a quiet technical journal: one reading column, concise archive rows, editorial body text, restrained color, and no sidebar. It borrows content-first principles from the reference sites without copying their branding, artwork, wording, or layout.
