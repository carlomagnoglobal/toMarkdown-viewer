# toMarkdown Viewer — User Guide

toMarkdown Viewer is the desktop app for reading, editing, and organizing Markdown notes and Obsidian-style vaults — and for converting anything else (PDF, DOCX, XLSX, EPUB, web pages, …) into Markdown. This guide covers everyday use; to install the app see [INSTALL.md](INSTALL.md), and for architecture/build details see [GUI.md](GUI.md).

## Contents

1. [Getting started](#getting-started)
2. [Opening files and vaults](#opening-files-and-vaults)
3. [Reading view](#reading-view)
4. [Localizing external links](#localizing-external-links)
5. [Editing](#editing)
6. [Dropping files into notes](#dropping-files-into-notes)
7. [Importing & converting (files and URLs)](#importing--converting-files-and-urls)
8. [Search, tags, and tasks](#search-tags-and-tasks)
9. [Navigation: switcher, graph, backlinks](#navigation-switcher-graph-backlinks)
10. [Text analysis & document statistics](#text-analysis--document-statistics)
11. [AI actions](#ai-actions)
12. [Export & sharing](#export--sharing)
13. [Themes & appearance](#themes--appearance)
14. [Settings](#settings)
15. [Keyboard shortcuts](#keyboard-shortcuts)
16. [Troubleshooting](#troubleshooting)

---

## Getting started

Launch the app (`cargo run -p to_markdown_gui`, or the installed bundle). You'll see a sidebar on the left and an empty content pane: **open a folder or file — or drop one onto the window**.

- A **folder** becomes your *vault*: the file tree, search, tags, tasks, graph, and wikilinks all work vault-wide.
- A **single file** opens for reading without vault features.

The app remembers recents, pinned notes, open tabs (per vault), window size, and — if enabled in Settings — reopens the last vault on launch.

## Opening files and vaults

| How | What it does |
| --- | --- |
| **Open Folder** button | Native picker; opens a vault |
| **Open File** button | Native picker; opens one document |
| Drag & drop onto the window | Folder → vault; `.md` → opens; other formats → converted to Markdown (see [Importing](#importing--converting-files-and-urls)) |
| Finder double-click on a `.md` | Opens in the app (file association) |
| Recents / Pinned / Known vaults | One click to reopen |

The tree filters out hidden and build directories and refreshes automatically when files change on disk. Right-click a tree item for **New note here / New folder / Pin / Rename / Copy path / Reveal in Finder / Delete**.

## Reading view

Markdown renders with headings, tables, task lists, footnotes, syntax-highlighted code, Obsidian callouts, `==highlights==`, KaTeX math, Mermaid diagrams, local images/PDF/audio/video, and `![[Note]]` transclusions.

- **Live reload** — edit the file in any external editor; the pane re-renders on save, keeping your scroll position.
- **Wikilinks** are clickable; hover one for an Obsidian-style page preview.
- **Checkboxes** in the reading view toggle the task in the file.
- **Image lightbox** — click an image to zoom it full-screen in a centered overlay; press Esc or click the overlay to close.
- **Reading-progress bar** — a thin bar at the top of the window shows how far you've scrolled through the document (hidden in print).
- **Table of contents** appears in the Files pane for documents with 2+ headings.
- The **status bar** shows words · characters · estimated tokens · read time. Click it for the full [Text analysis](#text-analysis--document-statistics).

## Localizing external links

In the reading view, you can convert external links and remote images to vault-local copies:

- **Right-click an external link or remote image** → **"Store in vault"** saves the resource (image or URL document) locally and rewrites the link in the note
- **Right-click → "Convert to markdown note"** converts a URL's content to Markdown, saves it as a new note, and rewrites the link as a wikilink
- **Palette command "Localize External Links…"** scans the entire note for external targets and shows a list with per-item actions (images default to Store, links default to Skip); processes sequentially and skips failed downloads with a toast message

Localizing is useful for building a self-contained, offline vault and for capturing web content that might disappear.

## Editing

Three view modes, cycled with the **Edit** button:

| Mode | Shortcut | Description |
| --- | --- | --- |
| Reading | — | Rendered document |
| **Live editing** | Cmd+E | Typora-style: every block rendered; click a paragraph to edit its Markdown in place; Escape or click away to re-render |
| **Split source** | Cmd+Shift+E | Raw Markdown beside an instant preview with synced scrolling |

### Live editing mode

Click any paragraph, heading, code block, or list to edit its Markdown source in place. The active block is highlighted. Press Escape or click elsewhere to re-render and move to the next block. Links, checkboxes, and tags stay interactive around the editing area — click a checkbox to toggle the task, or a wikilink to navigate.

### Split source mode

Raw Markdown in the editor pane on the left; rendered output on the right with **proportional scroll sync** — scroll the editor and the preview stays aligned. The editor shows **Rust-computed syntax highlighting** as a backdrop layer, highlighting:

- Headings, bold, italic, inline code
- Wikilinks (`[[…]]`)
- Links, blockquotes, list markers
- Fenced code blocks, frontmatter

Autosave fires silently **1.2 seconds after you stop typing**, preserving your position; the status bar shows "editing…" while unsaved changes exist.

### Shared editing features

Everything autosaves (debounced, atomic writes). Editing extras:

- **`[[` wikilink autocomplete**: Start typing `[[` in either live or split mode; a popup shows ranked matches (title prefix, substring, alias). Navigate with Arrow Up/Down, accept with Enter or Tab, close with Escape.
- **`#` tag autocomplete** at the caret
- **Formatting**: Cmd+B bold, Cmd+I italic, Cmd+Shift+X strikethrough, Cmd+Shift+K link; lists auto-continue; Tab indents lists and jumps table cells
- **Paste** an image → filed into the vault's attachment folder as an `![[embed]]`; paste a URL over selected text → Markdown link
- **Find & replace**, table skeleton insertion, YAML **Properties** editor, **Rename** with inbound-link rewriting
- **Zen mode** (Cmd+Shift+Z) hides the sidebar, tabs, and status bar; **typewriter mode** keeps the caret centered

## Dropping files into notes

When a note is open in the editor, you can drag files into the note area to add their content:

- **Image files** (PNG, JPG, GIF, WebP, SVG, BMP) are **instantly embedded** — copied into the vault's attachment folder and referenced as `![[image.png]]`
- **Other files & URLs** show a **four-action dialog**:
  - **Copy into vault & link** — save the file (or URL resource) to attachments and link it
  - **Link original location** — insert a link to the file or URL as-is
  - **Convert to new linked note** — convert to Markdown and save as a new note, link it with `[[Note]]`
  - **Convert inline** — embed the converted Markdown directly in the current note (files over 200 KB ask for confirmation)
  - Press **Esc** to cancel
- **Multi-file drops** apply a single action to all files
- **Safari/browser image drags** (raw image data) download and embed; if offline or the URL is unreachable, a link is inserted instead
- **Drops on the sidebar or with no note open** keep the original behavior (open the file or import as a new note)

## Importing & converting (files and URLs)

The **Import…** button (also Cmd+K → "Import") turns anything the converter pipeline understands into Markdown:

- **Choose file…** or **drag & drop** a PDF, DOCX, XLSX, PPTX, EPUB, EML, HTML, CSV, AsciiDoc, Org, LaTeX, … file
- **Paste a URL** into the field and press Enter — the page is fetched and converted (with YAML frontmatter metadata)

Where the result goes:

- **Vault open** → saved to `Imports/` (filenames deduped) and opened in a new tab
- **No vault** → a Save As dialog asks where to put it
- Tick **Save As…** in the Import panel to always choose the destination

Files over 10 MB in structured formats are refused with guidance (this protects the converter).

## Search, tags, and tasks

The sidebar has four panes: **Files · Search · Tags · Tasks**.

- **Search** modes: Full text, Tag (nested-prefix aware), Alias, Frontmatter field, and **Semantic** (vector similarity over the persistent `.tomarkdown` embedding index)
- **Tags** — vault-wide browser sorted by count; click to search
- **Tasks** — every checkbox task with state, due date, and source note

## Navigation: switcher, graph, backlinks

- **Quick switcher** — Cmd+O or Cmd+P; fuzzy-match by title or alias
- **Command palette** — Cmd+K; run any app action by name
- **Graph** — force-directed link graph (global or current note); drag nodes, filter, click to open
- **Note panel** (below the sidebar) — frontmatter properties, tags, backlinks with context, outgoing links, related notes (vector similarity), and unlinked mentions
- **Tabs** — Cmd+click opens in a new tab; Cmd+W closes; Cmd+1…9 switches; persisted per vault
- **Daily note** — Cmd+D creates/opens today's note

## Text analysis & document statistics

Two complementary views, both in the command palette:

- **Text Analysis: Tokens & Frequencies** — also opened by **clicking the status-bar counter**. A tabbed overlay:
  - **Summary** — words, distinct words, characters, spaces, exact **OpenAI (gpt-4o) token count**, **Anthropic/Claude token estimate**, tokenization method
  - **Words / Characters / Tokens** — complete frequency tables (rank, item, count, share %); nothing truncated; sticky headers and arrow-key tab switching
- **Document Statistics** — readability (Flesch reading ease, grade level), sentence metrics, top words; pair it with the **Keyword Highlights** toggle to see repetitions in the text

## AI actions

Set an Anthropic API key in Settings (Cmd+,), then use the palette: **Summarize**, **Suggest Tags**, **Translate…**, **Ask About Document…**. Results open in an overlay with **Copy** and (while editing) **Insert into note**.

## Export & sharing

| Action | Where |
| --- | --- |
| Export HTML (standalone, current theme + user CSS inlined) | Toolbar / palette / File menu |
| Print / save as PDF | Toolbar / Cmd+K |
| Copy as rich text (paste into email/docs) | Toolbar |
| Export DOCX (Word) / RTF | Palette / File menu |

**Print & PDF improvements** — the print stylesheet hides all UI chrome (sidebar, tabs, toolbars), sets optimal page breaks to avoid splitting code blocks, tables, and headings, and adjusts text sizing and spacing for print readability.

## Themes & appearance

**System / Light / Dark / Sepia**, switchable from the toolbar, Settings, palette, or View menu. All UI colors — including syntax highlighting, callouts, errors, and highlights — follow the active theme. A **custom CSS** file can be layered on top (persisted). Content font size is adjustable in Settings. Scrollbars are always visible and theme-tinted.

## Settings

Cmd+, opens Settings: theme, content font size (adjustable with Cmd+=/−/0; clamps to 11–24 px), line-height (Compact/Normal/Relaxed/Airy), Anthropic API key (stored locally), reopen-last-vault, open-notes-in-Live-mode default, and the known-vaults list (click to open, × to forget). All settings persist across app launches.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Cmd+O / Cmd+P | Quick switcher |
| Cmd+K | Command palette |
| ? | Keyboard shortcut cheat sheet |
| Cmd+E | Reading ↔ Live editing |
| Cmd+Shift+E | Split source editor |
| Cmd+S | Save now |
| Cmd+D | Daily note |
| Cmd+W · Cmd+1…9 | Close tab · switch tab |
| Cmd+B / Cmd+I / Cmd+Shift+X / Cmd+Shift+K | Bold / italic / strikethrough / link |
| Cmd+Shift+Z | Zen mode |
| Cmd+= / Cmd+− / Cmd+0 | Increase / decrease / reset text size |
| Cmd+, | Settings |
| Escape | Close overlay / commit block edit / close lightbox |
| Enter / Space | Activate the focused tree item, tab, or result (full keyboard navigation) |

*(Use Ctrl instead of Cmd on Linux/Windows.)*

## Troubleshooting

- **"Open a vault folder first"** — vault features (search, graph, wikilinks, tags) need a folder open, not a single file.
- **Import of a URL fails** — check the address includes `http(s)://`; some heavily scripted pages convert with less fidelity (the fetcher reads the served HTML).
- **AI actions error** — add an Anthropic API key in Settings.
- **Large file refused** — structured formats over 10 MB are rejected by design; convert externally or split the file.
- **Unsigned app warning (macOS)** — right-click → Open on first launch; signing/notarization is not configured.
- Errors appear as **toasts** (transient actions) or a red banner (document-level); both show human-readable messages.
