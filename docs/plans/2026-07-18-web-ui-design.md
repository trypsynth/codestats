# Tiny web UI for codestats-web

## Goal

A minimal page, served by `codestats-web` itself, that lets someone on any device
(desktop or phone) either paste a git URL or upload a zip and see the resulting
codestats report, without touching a terminal. Intended to be hosted at
codestats.rs pointed at a running `codestats-web` instance.

## Hosting shape

The page is served by the existing `codestats-web` binary, not a separate static
site. One new route, `GET /`, returns the page; the existing `POST
/api/analyze/zip` and `POST /api/analyze/git` routes are unchanged and are what
the page's JS calls. Same origin, so no CORS configuration needed.

## Implementation

- New file: `crates/web/static/index.html` — a single file containing structure,
  an inline `<style>` block, and an inline `<script>` block. No CSS/JS files, no
  build step, no npm/node_modules.
- Pulled into the binary at compile time via `include_str!`, exposed through a
  new `index` handler in `main.rs` that returns `Html(INDEX_HTML)`. The deployed
  artifact stays a single executable — nothing to ship alongside it.
- No new Cargo dependencies.

## Page behavior

- Two tabs, "Git URL" and "Upload zip", toggled by JS that shows/hides the
  matching `<form>`. Only one input mode visible at a time (keeps it usable on a
  phone screen).
- Submitting either form is intercepted (`preventDefault`) and dispatched via
  `fetch()`:
  - Git tab: `POST /api/analyze/git` with `{"url": "..."}` as JSON.
  - Zip tab: `POST /api/analyze/zip` with a `FormData` containing the picked
    file under the `file` field (matches the existing multipart handler).
- While the request is in flight: the Analyze button is disabled and shows
  "Analyzing…", since a git clone or zip extraction can take a few seconds.
- On success (response body is the existing `ReportData` JSON, unchanged):
  - A summary line: total files, total lines, total size (using the response's
    existing human-readable fields, e.g. `total_size_human`).
  - A table of languages sorted by lines (already the JSON's default order),
    each row showing name, lines, and a CSS bar: a `<div>` whose
    `style="width: {code_percentage}%"` renders as a colored bar — no chart
    library, no client-side percentage math, just reading fields that are
    already in the response.
- On failure (non-2xx response): parse the `{"error": "..."}` body the API
  already returns and show it inline near the form, replacing any previous
  result.

## Styling

Single centered column, system font stack (`-apple-system, sans-serif` etc, no
webfont download), generous touch-target sizing for the tabs and button, and a
horizontally-scrollable wrapper around the language table so it doesn't break
layout on a narrow phone screen. No media queries needed since the layout is a
single column at every width.

## Out of scope

- No client-side routing, no state persistence across reloads, no history of
  past analyses.
- No auth/rate-limiting on the page itself (the existing API guardrails — size
  caps, zip-slip rejection, URL scheme restriction, clone timeout — are what's
  carrying that weight; this is still an MVP).
- No dark/light theme toggle; the page just respects `prefers-color-scheme`.
