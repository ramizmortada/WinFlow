# WinFlow — Project Overview & Roadmap

WinFlow is a lightweight, fast Windows desktop utilities companion built with **Tauri v2**, **Rust**, **React 19**, **Tailwind CSS**, and **shadcn/ui** (~25 MB RAM footprint).

---

## Current Capabilities

### 1. Clipboard Manager
- **Automatic Capture:** Listens to Windows clipboard changes for plain text, code snippets, URLs, `#HEX` colors, and PNG/BMP screenshots.
- **Smart Deduplication & Pinning:** Re-copying an item bumps it to the top; pinned items persist across history clears and restarts.
- **Instant Paste:** Clicking an item or pressing `Enter` copies it, hides WinFlow, and simulates `Ctrl + V` into the active window.
- **Search & Filters:** Filter by **All**, **Text**, **Colors**, or **Images**, plus **Recent** and **Pinned** tabs.
- **Image Saving:** Save any captured screenshot directly to your Downloads folder.

### 2. Screen Color Picker (`Alt + C`)
- Captures a multi-monitor virtual desktop snapshot and displays a live magnifying loupe around the cursor.
- One-click copies the `#HEX` color code to the clipboard (and into WinFlow history).

### 3. File Explorer Context Menu Integration
- Registers user-level (`HKCU\Software\Classes`, no admin required) Windows File Explorer right-click entries:
  - **Copy All Names (WinFlow)** when right-clicking inside an open folder.
  - **Copy Selected Names (WinFlow)** when right-clicking one or more selected files/folders (batched cleanly across multi-select).
- Configurable in **WinFlow Settings**: enable/disable the Explorer menu and toggle **Include File Extensions**.

### 4. Desktop Tools (Tray & Header)
- **Black Screen:** Covers all monitors in pure `#000000` with a hidden cursor; press any key or click anywhere to exit.
- **Power / Shutdown Timer:**
  - **Actions:** Shutdown, Restart, Sleep, Lock, Log Out.
  - **Options:** *Graceful (do not force close apps)* and *Prevent system from going to sleep* while counting down.
  - **Scheduling:** *After a specific timespan* (`Hours / Minutes / Seconds` + `15m / 30m / 1h / 2h` presets) or *At a specific time of day*.

### 5. Snipping Tool (`Alt + S`)
- **Capture Modes:** **Region** (drag to crop), **Window** (hover & click any visible window), and **Fullscreen**.
- **Delay Timer:** `0s`, `3s`, `5s`, or `10s` delay selector on the floating capture bar.
- **Annotations:** Arrow, Rectangle, freehand Highlighter, Blur/Redact, Text, color & stroke picker, Undo, and Clear.
- **Actions:** **Copy** to clipboard (and WinFlow history), **Save** PNG to Pictures/Downloads, or **Pin to Screen** as an always-on-top floating reference window.

---

## Future Roadmap / Planned Features

### Other Potential Additions
- **Screen Text Extractor / OCR (`Alt + T`):** Extract text from any screen region using Windows' built-in OCR API.
- **More Explorer Copy Actions:** Copy Full Path(s) and Copy Folder Tree.
- **Text Transforms:** Paste as plain text, trim whitespace, change case, or pretty-print JSON.
- **Password Manager Auto-Ignore:** Respect Windows' `ExcludeClipboardContentFromMonitorProcessing` flag.
