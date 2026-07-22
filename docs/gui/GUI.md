# toMarkdown Viewer (desktop GUI)

Desktop viewer for Markdown, vaults, and every format the converters support (PDF, DOCX, HTML, EPUB, …) — a Tauri app in `gui/`, sharing all conversion/vault logic with the MCP server through the `to_markdown_mcp` library crate.

## Status

Phase 5 of the [roadmap](../planning/ROADMAP.md): foundation & viewer. Later phases add live file watching, Obsidian-grade vault navigation (backlinks, graph, search), Typora-style editing, and AI features.

## Current features

- **File tree sidebar** — open a folder (native picker, drag-and-drop, or recents); directories collapse/expand; hidden and build directories are filtered out; the tree refreshes automatically when files are added or removed
- **Rendered Markdown pane** — headings, tables, task lists, footnotes, strikethrough (pulldown-cmark); code and config files render as fenced code blocks with detected language
- **Rendering fidelity** — syntect syntax highlighting (theme-aware), local images/PDF/audio/video inlined as data URLs (`![[image.png]]` embeds render as media), Obsidian callouts (incl. folded), `==highlights==`, `%%comments%%` stripped, `![[Note]]`/`![[Note#Heading]]` transclusion blocks, KaTeX math (`$…$`, `$$…$$`) and Mermaid diagrams via vendored libraries (`gui/ui/vendor/`, no CDN)
- **Any supported format** — non-Markdown files (PDF, DOCX, XLSX, EPUB, HTML, …) are converted to Markdown by the shared pipeline before rendering
- **Live preview** — the open file is watched (`notify`); edit it in any external editor and the pane re-renders on save, preserving your scroll position (Marked 2 style)
- **Table of contents** — clickable, indentation-per-level, generated from the rendered headings
- **Themes** — System / Light / Dark / Sepia, plus a user CSS file that layers on top (persisted and reloaded on start)
- **Export** — standalone styled HTML (theme + user CSS inlined), Print / save-as-PDF via the system dialog, and copy-as-rich-text to paste into email/docs
- **Stats footer** — word count, character count, estimated read time
- **Recent files & folders** — persisted locally, one click to reopen
- **Wikilink navigation** — `[[target#heading|alias]]` links are clickable and resolve with Obsidian's shortest-path rules; embeds appear as ⧉ links; `.canvas` boards render as structured Markdown
- **Note panel** — frontmatter properties, tags (click to search), and backlinks with source context for the open note
- **Vault search tab** — full text, tag (nested-prefix aware), alias, or frontmatter field
- **Tags & Tasks tabs** — vault-wide tag browser sorted by count; all checkbox tasks with state, due date, and source
- **Graph view** — force-directed link graph (global or current-note local), drag nodes, click to open
- **Quick switcher** — Cmd/Ctrl+O (or P) fuzzy-opens notes by title or alias
- **Live editing (Typora-style)** — vault notes open with every block rendered; click any paragraph/heading/fence to edit its Markdown in place, Escape or click away to re-render; autosaves; links/tags/checkboxes stay interactive around the active block. Three modes: Reading / Live (Cmd+E) / Split source (Cmd+Shift+E)
- **Split editing** — source + instant preview; autosave (atomic writes, debounced), native undo/redo, find & replace, table skeleton insertion; Tab jumps table cells
- **Vault-aware authoring** — `[[` wikilink and `#` tag autocomplete at the caret, paste an image to file it in the vault's attachment folder as an `![[embed]]`, click checkboxes in the reading view to toggle tasks in the file
- **Note management** — New Note (folder-in-title supported, templates via vault config), Rename with inbound wikilink rewriting, YAML frontmatter editor with validation
- **Intelligence** — Related-notes in the note panel, a Semantic (vector) search mode using the persistent `.tomarkdown` embedding index, and AI actions (summarize, suggest tags, translate, ask about the document) when an Anthropic API key is set in Settings; results open in an overlay with Copy / Insert-into-note
- **Editor UX** — synced scrolling between editor and preview, interactive edit preview (checkboxes, wikilinks), Zen and typewriter modes, formatting shortcuts (Cmd+B/I, link, strikethrough, list indent, auto-continued lists), paste-URL-as-link, selection stats
- **Writing insight** — document statistics overlay (readability, top words) and a keyword-repetition highlighter; hover a wikilink for an Obsidian-style page preview
- **Vault workflows** — note tabs (Cmd+click, Cmd+W, Cmd+1..9, persisted per vault), daily note (Cmd+D), new-from-template, right-click file management in the tree (new/rename/delete/reveal/pin), clickable inline #tags, outgoing links and unlinked mentions in the note panel, pinned notes, graph filtering, and a multi-vault manager with reopen-on-launch
- **Command palette** — Cmd/Ctrl+K runs any app action by name
- **Settings** — Cmd/Ctrl+, for theme, content font size, API key, live-editing default, reopen-last-vault, and the known-vaults list
- **Native integration** — real macOS menu bar (File/Edit/View/Window with native Edit roles), `.md` file associations (Finder double-click opens in the app), window size/position remembered across launches
- **Export** — HTML, Print/PDF, copy-rich-text, plus DOCX (Word) and RTF via the palette or File menu (text-level fidelity; images omitted)
- **Import / convert** — the Import… toolbar panel and drag-and-drop convert any pipeline-supported file (PDF, DOCX, XLSX, EPUB, EML, HTML, …) or a pasted URL to Markdown; results save into the vault's `Imports/` folder (deduped) and open in a new tab, or through a Save As dialog when no vault is open
- **Text analysis** — click the status-bar counter (or palette) for a tabbed overlay with exact OpenAI token counts, an Anthropic estimate, and complete word/character/token frequency tables (rank, count, share %); the status bar itself shows a live token estimate
- **Feedback & polish** — toast notifications (success/error/info), busy spinners on long actions, friendly error messages, empty-state hints in every pane, always-visible themed scrollbars
- **Accessibility** — dialogs are ARIA-labeled with focus trapping and focus restore; the tree, tabs, results, and rows are keyboard-focusable and Enter/Space-activatable; visible focus outlines
- **Theme-safe colors** — errors, callouts, highlights, shadows, and syntax highlighting are theme-token driven; light syntax rules are hard-scoped so dark mode can't inherit unreadable colors

See the [User Guide](USER_GUIDE.md) for day-to-day usage and the full shortcut list, and the [Install Guide](INSTALL.md) for Windows/macOS/Linux installation.

## Build & run

The GUI is excluded from default workspace builds (`cargo build` still builds only the MCP server). Build it explicitly:

```bash
cargo build -p to_markdown_gui            # compile
cargo run -p to_markdown_gui              # launch the viewer
```

Packaging: `cargo tauri build` from `gui/` produces the `.app`/`.dmg` (tauri-cli required: `cargo install tauri-cli --locked`). CI can build macOS/Linux/Windows bundles via the `GUI build` workflow (`.github/workflows/gui-release.yml`, manual dispatch or `gui-v*` tags). Signing/notarization requires Apple Developer certificates and is not configured — right-click → Open on first launch.

## Manual test checklist

### Phase 1 polish

- **Image lightbox**: Click a note with images; click an image to zoom full-screen in a centered overlay; press Esc or click outside to close cleanly
- **Reading-progress bar**: Open a long note; scroll to the middle, bar should be half-width; scroll to bottom, bar should span full width; verify it is hidden in print
- **Keyboard cheat sheet**: Press `?` in the reading view to open shortcuts overlay grouped by category; press Esc or click overlay to close; verify it does not open when typing in inputs
- **Text size controls**: Cmd/Ctrl+= to increase, − to decrease, 0 to reset; verify size clamps to 11–24 px; restart app, size should persist
- **Line-height setting**: Cmd+, opens Settings; find Line height dropdown (Compact/Normal/Relaxed/Airy); change it; verify setting persists across restart
- **Print / PDF output**: Cmd+K → "Print / Save as PDF" on a note with code, tables, and headings; verify PDF has no chrome (sidebar/tabs/statusbar hidden), no mid-block page breaks in code/tables, full-width text, and subtle links (inherit color, underlined)

### Phase 2 — Rust-first hybrid editing

- **Backdrop alignment (split mode)**: Cmd+Shift+E to open split editing; verify the highlighted syntax backdrop aligns precisely with the editor text (headings, bold, code, wikilinks). Emoji note: if bold or italics drift visually, adjust font-weight in CSS.
- **Live-block highlighting**: Cmd+E to enter live mode; click a paragraph to edit; verify the editing block is highlighted with a subtle background color and remains distinct from rendered blocks around it.
- **`[[` wikilink autocomplete (both modes)**: In split mode, type `[[` in a line; autocomplete popup appears ranked by title prefix, then substring, then alias. Navigate with Arrow Up/Down, accept with Enter or Tab, close with Escape. Repeat in live mode.
- **Silent autosave timing (split mode)**: Type in split mode; the status bar shows "editing…" while unsaved. Wait 1.2 seconds after typing stops; status should update to "saved" with a timestamp, without a toast notification.
- **Scroll sync (split mode)**: Open a long note in split mode; scroll the editor (left pane); the preview (right pane) should scroll proportionally to keep alignment, and vice versa.
- **Render cache snappiness (long notes)**: Open a note with 50+ paragraphs; switch between reading, live, and split modes; verify UI remains responsive and mode transitions complete without lag or visual glitches.

### Phase 3 — Drop-and-localize

- **Finder file drop on note**: Drag a PDF or DOCX file from Finder onto the note content area; dialog appears with four actions; select "Convert inline" and verify content is inserted.
- **Finder file drop on sidebar**: Drag a file onto the sidebar (tree area); verify the old import-as-new-note behavior occurs (no dialog).
- **Image file instant embed**: Drag a PNG/JPG from Finder onto the note; verify it is immediately embedded as `![[filename]]` with no dialog, file is copied to attachments folder.
- **Safari image drag**: Drag an image from a webpage onto the note; verify it downloads and embeds; test offline by unplugging network — verify fallback inserts a plain link with "Could not download" toast.
- **Multi-file one-action**: Drag three mixed files (image + PDF + DOCX) onto note; image embeds instantly; dialog appears for the others offering one action that applies to both.
- **Esc cancels drop**: Drag a file onto the note, dialog appears; press Esc; verify dialog closes and nothing is inserted.
- **Oversized inline confirm**: Drag a 250+ KB file onto note, select "Convert inline"; verify a confirmation dialog appears ("Converted text is N KB — insert anyway?"); confirm to insert the content.
- **Right-click store remote image**: In reading view, right-click a remote image (http/https); select "Store in vault"; verify the file downloads, image now loads from local attachments, link is rewritten.
- **Right-click convert remote link**: Right-click an external link in the reader; select "Convert to markdown note"; verify a new Imports/ note is created, link becomes a wikilink.
- **Whole-note localize with dead URL**: Palette → "Localize External Links…" on a note with a 404 URL; verify the dead link is listed, attempt fails with a toast, note is untouched for that link.
- **devicePixelRatio hit-test on retina**: On a Retina Mac (2x DPI), drag a file onto the note; verify drop dialog appears correctly (position is adjusted by device pixel ratio, not clipped off-screen).

## Word Graph

The Word graph visualizes relationships between words in your vault based on 
co-occurrence—words that appear together in notes. Use it to discover potential 
super-links: tight word clusters indicate related notes that should be linked.

### How to use
1. Open a note, click the Graph button
2. Click "Word" to switch to word-relationship view
3. Larger words appear more frequently; edge thickness shows co-occurrence strength
4. Click a word to highlight all notes containing it
5. Zoom with mouse wheel, touch pinch, or +/− buttons; press 0 to reset

### Technical details
- Index built automatically on app startup (if fresh) or when opening Word tab (delta update)
- Top 200 words by frequency (excluding common stopwords like "the", "and")
- Co-occurrence edges shown for word pairs appearing together in 2+ notes
- Index stored locally in `.tomarkdown_word_graph.db` SQLite database

### Word Graph Manual Tests

- [ ] Open Word tab, verify graph renders with word nodes sized by frequency
- [ ] Zoom with mouse wheel, verify per-view zoom independent
- [ ] Zoom with +/− buttons, verify zoom increments by 10%
- [ ] Press 0 to reset zoom, verify returns to fit-all
- [ ] Click a word node, verify related notes highlight in sidebar
- [ ] Hover word node, verify tooltip shows stats
- [ ] Modify a note, open Word tab, verify index updates within 1s
- [ ] Verify stopwords (the, and, is) excluded from graph

## Architecture

- `gui/src/main.rs` — Tauri commands: `list_tree` (recursive, sorted, filtered), `open_file` (convert → Markdown → HTML), `pick_folder`/`pick_file` (native dialogs)
- `gui/ui/index.html` — single-file vanilla JS frontend (no npm/bundler); talks to Rust via `window.__TAURI__` (`withGlobalTauri`)
- `src/lib.rs` / `src/pipeline.rs` — the shared library: all converter modules plus `convert_any_to_markdown` with the large-file guardrails
