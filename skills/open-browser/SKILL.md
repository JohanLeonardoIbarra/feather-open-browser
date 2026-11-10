---
name: open-browser
description: Control the local Open Browser app using its authenticated localhost API. Dedicated agent-exclusive browser environment for inspecting, developing, and debugging web applications.
---

# Open Browser (Speed-Optimized Agent Protocol)

Open Browser is an ultra-lightweight browser environment dedicated exclusively to AI agents. It launches in standby with 0 spaces and spawns ephemeral Spaces strictly on-demand.

---

## ⚡ High-Speed Execution Rules (Zero-Overthinking)

To ensure maximum speed, low latency, and zero redundant reasoning turns, follow these strict rules:

### 1. One-Shot Space Creation
- **Always** provide the initial target `url` when creating a space:
  ```json
  { "name": "Task Name", "url": "https://example.com" }
  ```
- This navigates the tab immediately during creation, eliminating an extra navigation turn.

### 2. Action Tools Already Return Fresh Snapshots
- `browser_click`, `browser_fill`, `browser_press`, `browser_wait`, and `browser_run_actions` **already return the updated page snapshot and valid refs (`e1`, `e2`, ...)** in their response.
- 🛑 **NEVER call `browser_snapshot` immediately after an action tool.** It wastes an entire reasoning turn and duplicates token context. Read the snapshot directly from the action's result!

### 3. Batch Predictable Actions with `browser_run_actions`
- Do not execute form interactions sequentially across separate turns. Combine them into a single `browser_run_actions` call:
  ```json
  {
    "spaceId": "...",
    "tabId": "...",
    "actions": [
      { "type": "fill", "ref": "e2", "value": "search query" },
      { "type": "press", "key": "Enter" }
    ]
  }
  ```
- Supported action types: `navigate` (`url`), `click` (`ref`), `fill` (`ref`, `value`), `press` (`key`), `wait` (`ms`), `back`, `forward`, `reload`. Maximum 32 actions per batch.

### 4. Direct URL Formulation Over UI Searching
- If the website supports query parameters (e.g., `/search?q=query` or `/search/?name=query`), formulate and navigate to the URL directly with `browser_navigate` instead of clicking search inputs, typing, and submitting.

### 5. Zero Redundant Waits
- Standard HTTP/HTTPS navigations and page transitions are automatically awaited by the browser.
- **Do not** insert `browser_wait` or sleep actions unless a page uses heavy client-side JavaScript rendering where required elements have not yet mounted.

### 6. Quick Target Identification
- To find the "latest chapter", "newest item", or primary action:
  - Check the top-most link in the list or scan for tags like `"Nuevo"`, `"Latest"`, `"Recent"`, or highest numeric indicators.
  - Immediately click its ref (`browser_click({ ref: "e..." })`) without over-analyzing irrelevant elements.

---

## Tool Reference

### Lifecycle
- `browser_create_space`: Spawn an isolated Space. Always pass `{ name, url }`.
- `browser_close_space`: Close space and clean up browsing data when the goal is achieved.
- `browser_open_tab` / `browser_close_tab`: Manage tabs within an existing space.

### Interaction (All return updated snapshot!)
- `browser_run_actions`: **Preferred**. Batch multiple actions (`fill`, `press`, `click`, `wait`, `navigate`) in one call.
- `browser_click`: Click an element by ref (`e1`, `e2`, ...).
- `browser_fill`: Set value on text input/textarea.
- `browser_press`: Send keyboard key (`Enter`, `Tab`, `Escape`, `Backspace`, etc.).
- `browser_navigate`: Go to an absolute HTTP(S) URL.
- `browser_wait`: Wait up to 5000ms for delayed client-side DOM rendering.

### Observables & Diagnostics (Use only when necessary)
- `browser_snapshot`: Read DOM tree and interactive refs (only needed on initial space load if not already provided).
- `browser_screenshot`: Capture visual PNG screenshot (use when checking layout or visual bugs).
- `browser_console_logs`: Retrieve client-side console logs and runtime JavaScript errors.
- `browser_network_requests`: Inspect network requests and response status codes.
- `browser_get_html`: Fetch full HTML or specific ref outerHTML.

---

## Standard 3-Step Task Pattern

```
Turn 1: browser_create_space({ name: "Research", url: "https://site.com" })
        -> Receive initial snapshot with refs.
Turn 2: browser_run_actions([ { type: "fill", ref: "e1", value: "query" }, { type: "press", key: "Enter" } ])
        -> Receive search results snapshot with refs.
Turn 3: browser_click({ ref: "e5" })
        -> Done! Selected target loaded.
```
