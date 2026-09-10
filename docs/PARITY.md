# Migration parity contract

The existing `indianboy` application remains the behavioral baseline. This repository is allowed to change presentation, but not silently remove existing capabilities.

## Current vertical slice

- [x] Rust native site builder
- [x] Rust/WASM `workers-rs` fetch handler
- [x] Cloudflare Static Assets integration
- [x] All 13 existing posts migrated
- [x] YAML frontmatter and generated reading time
- [x] GFM tables, task lists, strikethrough, links, images, footnotes
- [x] Fenced code blocks with language-aware syntax highlighting
- [x] Dollar-delimited math, rendered by the small KaTeX browser enhancement
- [x] RSS, sitemap, robots, canonical article paths
- [x] Giscus repository/category/pathname configuration
- [x] Theme toggle, local persistence, copy-link enhancement
- [x] Archive tag counts, collapsed filters, and multi-tag OR filtering
- [x] Homepage projects, work history, social links, and contact sections
- [x] Existing public assets copied into the generated site
- [x] Markdown+ `[[note: ...]]` directive for the existing sidenote
- [x] Markdown+ `[[cover: src | alt | caption]]` directive
- [x] Mermaid fenced blocks progressively render to SVG
- [x] No article sidebar; single reading column

## Remaining parity work

- [x] Add generated heading IDs and long-article contents navigation
- [x] Tune the Rust syntax theme to the new warm editorial palette
- [x] Add route smoke coverage for every route and legacy redirect
- [x] Add Markdown+ unit fixtures for notes, covers, Mermaid, math, tables, and duplicate headings
- [x] Add GitHub Actions validation for Rust, WASM, generated output, and route smoke tests
- [x] Test light/dark/system behavior, reduced motion, keyboard navigation, and narrow layouts
- [ ] Verify all existing Giscus discussions resolve to the same pathname
- [x] Preserve source date `2025-06-31` in HTML and normalize it to `Tue, 01 Jul 2025` in RSS, matching the legacy JavaScript behavior

## Explicit presentation changes

- No global or article sidebar.
- No terminal grid, scanlines, prompt labels, or Catppuccin-specific visual identity.
- One editorial reading column with compact archive rows.
- Decorative Motion entrance effects and rotating footer copy are not part of the new visual direction unless reintroduced as progressive enhancements.
