# STATUS — webtui

## Confirmed working

- Headless Chrome launches under a persistent profile at `~/.local/share/webtui/profile`
  and is closed on exit; no orphaned Chrome after `q` (checked with `pgrep -f webtui/profile`).
- `extract.js` returns a structured block list from the settled DOM. Verified on three
  inputs: `example.com` (3 blocks), `demo.playwright.dev/todomvc` (client-rendered), and
  `news.ycombinator.com` (351 blocks, 4946 on an item page).
- **JS really runs.** The TodoMVC placeholder "What needs to be done?" appears in the
  extraction and is absent from `curl`'d HTML (0 matches). That is the known-good case
  separating "Chrome rendered it" from "we fetched some markup".
- TUI renders, scrolls, focuses by `Tab` or by number, and clicks. Clicking HN comment
  link `[17]` moved the URL to `item?id=…` and the comment text rendered.
- Typing into a React controlled input works via the native value setter: "buy oat milk"
  was added to the TodoMVC list and the count moved to "1 item left".
- JS console returns values (`document.querySelectorAll('a').length` -> 4160) and renders
  syntax errors as errors rather than as an empty value.
- `cargo test` — 5 render-layer tests pass. Release build is warning-free.

## Known broken

- Nothing known broken. The rough edges below are v1 scope decisions, not defects.

## Next Up

- Textless icon links render as `(icon)` and are still focusable, which is noisy on
  link-dense pages like HN.
- Scroll position is line-based over wrapped text, so `G` overshoots on long pages.
- `settle()` is a fixed 600ms after a click. A readyState/network-idle poll would be
  both faster and more correct.

Full open list lives in `TASKS.md`.

## Decision log

### 2026-08-14

- Decided: build this rather than adopt a tool. Carbonyl is abandoned upstream (last
  commit Feb 2023) and renders pixels we do not want; Browsh has no release since Jan
  2024; Chawan is alive but is its own browser with no external control hook; w3m/lynx
  have no JS engine. `browser-use/terminal` is the nearest packaged thing but is built to
  drive an agent, not to read.
- Decided: text-and-structure fidelity only. That is what makes the project tractable —
  pixel fidelity needs a browser engine drawing into the terminal, structure needs only a
  rendered DOM, so Chrome renders off-screen and we read the DOM back over CDP.
- Decided: address elements by a stamped `data-webtui-id` attribute rather than by CDP
  node handles, so a reference survives a re-render. Consequence: ids are re-issued on
  every extraction, so focus is cleared whenever the document is re-read.
- Decided: write inputs through the native `value` setter plus synthetic `input`/`change`
  events. A plain `el.value = x` is silently ignored by React and other controlled
  components — the failure would look like a successful write.
