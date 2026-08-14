# webtui

A terminal web browser: real headless Chrome renders the page off-screen, the settled
DOM comes back over CDP, and ratatui draws its structure. Text and structure only, no
images and no CSS layout — the point is reading, interacting, and scripting, not
pixel fidelity.

## Stack

Rust 2021, `ratatui` 0.30 + `crossterm` 0.29, `chromiumoxide` 0.9 (CDP), `tokio`.
Chrome is discovered on PATH / in `/Applications`; the profile lives at
`~/.local/share/webtui/profile` so logins persist between runs.

## Commands

```
cargo build --release
cargo test                       # render layer only; it is the sole pure module
./target/release/webtui <url>
```

## Layout

| File | Role |
|---|---|
| `src/browser.rs` | Owns Chrome and the one page. `goto`/`extract`/`click`/`type_into`/`submit`/`eval`. Every method is a thin wrapper over `Page::evaluate`. |
| `src/extract.js` | The DOM walk. Stamps `data-webtui-id` on interactive nodes and returns a flat block list. Included via `include_str!`. |
| `src/extract.rs` | `Document`/`Block`/`Kind` deserialization of that JSON. |
| `src/render.rs` | Pure `Document -> Vec<Line>`. No state, so it is the only unit-tested module. |
| `src/app.rs` | Event loop, key handling, layout, prompts. |

## Constraints

- **Element identity is `data-webtui-id`, not a CDP node handle.** Ids are re-issued on
  every extraction, so focus is cleared whenever the document is re-read.
- **Inputs are written through the native value setter**, not `el.value = x`. React and
  other controlled components ignore a plain assignment.
- **Chrome must be closed explicitly** on exit. It is a child process the terminal does
  not clean up; a leaked headless Chrome is this project's obvious silent failure.
- After a click there is no way to tell navigation from an in-place DOM mutation, so
  `settle()` waits a fixed 600ms and then re-extracts.

## Keys

`:` url · `` ` `` js console · `j`/`k`/`space`/`g`/`G` scroll · `Tab` cycle focus ·
digits+`Enter` focus by number · `Enter` click · `i` type into focused field ·
`r` re-extract · `q` quit
