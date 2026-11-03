---
name: Open Browser
description: A Linux-first Electron browser with compact Space wayfinding and a page-first canvas.
colors:
  foreground: "#173648"
  route-ink: "#12384b"
  route-ink-raised: "#1e4a5b"
  button-ink: "#17495a"
  route-ground: "#e8efec"
  route-ground-raised: "#f7f9f6"
  route-band: "#e5eeea"
  tab-rail: "#dce6e3"
  route-line: "#c4d3ce"
  route-muted: "#4f686c"
  personal-marker: "#a7c9d0"
  route-teal: "#2e9f91"
  route-coral: "#ef654d"
  route-amber: "#e5b249"
  field-white: "#ffffff"
  inverse-light: "#f2f6f3"
  route-overlay: "#0d283577"
typography:
  display:
    fontFamily: '"Public Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "32px"
    fontWeight: 620
    lineHeight: 1.2
    letterSpacing: "-0.035em"
  display-compact:
    fontFamily: '"Public Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "26px"
    fontWeight: 620
    lineHeight: 1.2
    letterSpacing: "-0.035em"
  headline:
    fontFamily: '"Public Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.55
  title:
    fontFamily: '"Public Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "13px"
    fontWeight: 690
    letterSpacing: "0.015em"
  body:
    fontFamily: '"Public Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "12px"
    fontWeight: 400
  label:
    fontFamily: '"Public Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "11px"
    fontWeight: 400
    letterSpacing: "0.03em"
rounded:
  xs: "4px"
  sm: "5px"
  md: "6px"
  lg: "8px"
  xl: "10px"
  round: "50%"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  2xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.button-ink}"
    textColor: "{colors.inverse-light}"
    rounded: "{rounded.sm}"
    padding: "0 13px"
    height: "31px"
  button-primary-hover:
    backgroundColor: "{colors.route-ink-raised}"
  button-search:
    backgroundColor: "{colors.route-ink}"
    textColor: "{colors.inverse-light}"
    rounded: "{rounded.sm}"
    padding: "0 14px"
    height: "36px"
  button-search-hover:
    backgroundColor: "{colors.route-ink-raised}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.route-ink-raised}"
    rounded: "{rounded.md}"
    size: "30px"
  space-chip:
    backgroundColor: "transparent"
    textColor: "{colors.inverse-light}"
    rounded: "{rounded.sm}"
    padding: "4px 10px"
  space-chip-active:
    backgroundColor: "{colors.route-ink-raised}"
    textColor: "{colors.field-white}"
    rounded: "{rounded.sm}"
    padding: "4px 10px"
  tab-select:
    backgroundColor: "transparent"
    rounded: "5px 5px 0 0"
    padding: "0 4px"
    height: "100%"
  address-input:
    backgroundColor: "{colors.field-white}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "0 13px"
    height: "32px"
  newtab-search:
    backgroundColor: "{colors.field-white}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "0 8px 0 16px"
    height: "54px"
---

# Design System: Open Browser

## Overview

**Creative North Star: “Routes through the web”**

Open Browser borrows public-transit wayfinding to distinguish Personal browsing from agent Spaces. A deep blue-green browser shell, cool pale surfaces, and one thin route line make Space context quickly scannable without competing with the active page. The new-tab view keeps that same restraint: one centered search action, not a dashboard. Public Sans is self-hosted as a variable WOFF2 under SIL Open Font License 1.1; there are no raster assets, and the mark and controls are inline SVG while status markers are CSS shapes.

This is a Linux-first Electron desktop window with an HTML/CSS renderer; PRODUCT.md's `web` platform value describes that renderer, not a mobile target. “Open Browser” remains a working product name rather than a confirmed brand identity.

**Key Characteristics:**
- Transit-inspired route line and status nodes are browser wayfinding, not page content.
- Compact Public Sans browser chrome sits above a light, page-first canvas; system font stacks are fallbacks only.
- Space state is communicated with both status color and explicit text.
- Shared colors come from `src/ui/tokens.css`; surfaces stay mostly flat with fine borders.

## Colors

The canonical palette pairs deep blue-green structure with cool pale grounds; teal, coral, and amber distinguish Space status, while Personal uses a quiet blue-grey marker.

### Primary
- **Deep Route Ink** (`route-ink`): masthead and the new-tab search action.
- **Raised Route Ink** (`route-ink-raised`): selected/hovered Space surfaces and filled-action hover states.
- **Action Ink** (`button-ink`): filled address-row navigation action.
- **Route Teal** (`route-teal`): ready state and focused address/search surfaces.

### Secondary
- **Signal Coral** (`route-coral`): working state and keyboard focus outlines.

### Tertiary
- **Station Amber** (`route-amber`): loading state.

### Neutral
- **Browser Foreground** (`foreground`): primary text on pale surfaces.
- **Browser Ground** (`route-ground`) and **Raised Ground** (`route-ground-raised`): browser chrome and page/tab surfaces.
- **Route Band** (`route-band`) and **Tab Rail** (`tab-rail`): Space activity and tab-strip backgrounds.
- **Route Line** (`route-line`) and **Route Muted** (`route-muted`): borders, connectors, subdued labels, and placeholders. Route Muted clears WCAG AA for normal text on the route ground, raised ground, route band, and tab rail; its lowest contrast on these surfaces is 4.67:1 on the tab rail.
- **Personal Marker** (`personal-marker`): Personal indicators in the selector, route rail, and repeated status.
- **Field White** (`field-white`) and **Inverse Light** (`inverse-light`): input surfaces and text on dark controls.
- **Route Overlay** (`route-overlay`): translucent dialog backdrop.

**The Four-State Wayfinding Rule.** Personal uses `personal-marker`; ready is teal, working is coral, and loading is amber. Pair each state with its visible text. A working Space selector dot is solid coral with no halo; the selected route node is a hollow, status-colored ring.

## Typography

**Display Font:** Public Sans (self-hosted variable WOFF2; `ui-sans-serif`, `system-ui`, `-apple-system`, `BlinkMacSystemFont`, and `"Segoe UI"` are fallbacks)  
**Body Font:** the same Public Sans-first stack  
**Label/Mono Font:** no separate mono face is used.

**Character:** Public Sans keeps browser controls compact and legible while giving chrome and the new-tab view one consistent voice. The new-tab heading provides the single prominent display moment; there is no decorative display face.

### Hierarchy
- **Display** (weight 620, 32px, line-height 1.2, tracking -0.035em): desktop new-tab heading.
- **Display Compact** (weight 620, 26px, line-height 1.2, tracking -0.035em): narrow new-tab heading at max-width 620px.
- **Headline** (weight 400, 14px, line-height 1.55): explanatory copy below the heading.
- **Title** (weight 690, 13px, tracking 0.015em): product wordmark and compact strong labels.
- **Body** (weight 400, 12px): hints and compact browser copy.
- **Label** (weight 400, 11px, tracking 0.03em): Space and API metadata; route labels use the same compact scale with stronger weights where needed.

## Layout

The desktop browser reserves 221px for fixed chrome: a 42px masthead, 37px tab row, 48px address/navigation row, and 94px route band. The active web page takes the remaining canvas. The route band aligns its heading, horizontally scrollable node track, and repeated active status in three columns; the renderer displays up to the first eight Spaces.

At widths up to 920px, browser-chrome gaps and padding tighten, the add-Space and API text labels collapse, and the route grid narrows. The new-tab content sits in a centered 620px-capped column; at widths up to 620px, its padding tightens and the heading takes the 26px responsive display step. The declared new-tab minimum width is 320px. These are desktop-window adaptations, not a mobile layout. Spacing follows a recurring 4px rhythm, with common steps at 4, 8, 12, 16, 20, and 24px.

**The Page-First Route Rule.** Keep browser controls and Space wayfinding in fixed chrome; the live page remains the main canvas, and the route rail reports context without replacing page content.

## Elevation & Depth

Browser chrome relies on tonal bands and thin borders rather than resting shadows. The new-tab search gets a subtle lift; modal dialogs use a deeper neutral-black shadow. Status markers remain flat, with state conveyed by fill or ring geometry and no glow or halo.

### Shadow Vocabulary
- **New-tab search lift** (`box-shadow: 0 4px 16px #0000000c`): subtle separation around the centered search form.
- **Dialog depth** (`box-shadow: 0 18px 54px #0000002e`): separation for modal dialogs.

**The Neutral Depth Rule.** Keep resting chrome flat; reserve shadows for the search form and dialogs, never for Space status markers.

## Shapes

Compact controls use gently rounded corners (4–6px); address/search surfaces use 8px, and dialogs use 10px. Status dots and route nodes are circular, connected by a thin straight line. Fine one-pixel borders define surface edges. Tabs have rounded upper corners and a square lower edge; narrow rounded scroll thumbs keep overflow tracks unobtrusive.

## Components

### Buttons
Browser actions are compact and direct; navigation icons stay quiet.
- **Shape:** filled actions use 5px corners; toolbar icon controls use 6px and a 30px square target.
- **Primary:** address navigation uses the dark action surface (31px high, 13px horizontal padding); new-tab search uses the deeper route ink (36px high, 14px horizontal padding).
- **Hover / Focus:** filled actions lighten on hover. Buttons use a 2px coral keyboard outline with a 2px offset; disabled navigation controls use 45% opacity and a not-allowed cursor.
- **Secondary / Ghost:** toolbar controls are transparent at rest, with a pale border/surface on hover. Add-Space is outlined; close-Space uses coral for its icon.

### Chips
Space chips act as compact wayfinding controls, not decorative tags.
- **Style:** transparent at rest, with a small state dot, pale text, and transparent border.
- **State:** hover and selection use a raised blue-green surface; the active chip adds a teal lower edge and stronger label. Personal, ready, working, and loading dots follow the four-state rule.

### Inputs / Fields
The address field is a flexible 32px-high white row control with a fine border and 8px corners. The new-tab search is a 54px-high white form with an inline magnifier and inset action button. Address focus and search `:focus-within` use teal border/outline emphasis; placeholders retain full opacity.

### Navigation
The masthead Space selector and tab strip preserve familiar desktop browser behavior. Space chips expose selected state; the active tab uses a pale surface, close control, and clipped title. The inner `.tab-select` button has authored reset and state styling: transparent at rest, four-pixel horizontal inset, rounded upper corners, and a pale hover surface; it does not rely on native Chromium button chrome.

### Route Status Rail
The signature rail is a horizontal connector with circular nodes and paired Space-name/status labels. Resting nodes are 13px; the selected node grows to 17px with a 3px border and a hollow, status-colored center. Non-selected ready, working, and loading nodes are filled teal, coral, and amber; Personal uses the blue-grey marker. The selector's working dot is a solid coral dot, without a halo. The right-edge status repeats the state in text and a dot. The rail is browser-owned wayfinding; visited pages remain ordinary web content.

### New-Tab Search
The centered search form combines a fine border, white surface, subtle neutral lift, inline magnifier, and compact dark submit button. Focus-within adds teal border and outline emphasis. Its 620px-capped column keeps the prompt, search action, and address hint together without dashboard cards.

## Do's and Don'ts

### Do:
- **Do** use `src/ui/tokens.css` as the canonical shared palette; keep interface colors tied to its custom properties.
- **Do** pair Personal/ready/working/loading marker colors with explicit status text; use the blue-grey Personal marker and teal/coral/amber for ready/working/loading.
- **Do** keep the active web page as the main canvas, with browser controls and Space wayfinding in the fixed chrome.
- **Do** retain visible coral keyboard-focus treatments and the reduced-motion override for transitions and animations.
- **Do** reserve elevation for the new-tab search shadow and neutral-black dialog depth.

### Don't:
- **Don't** add a glow or halo to working state; use the solid coral selector dot or the coral active-route ring.
- **Don't** turn the route rail into page content or browser chrome into a task dashboard.
- **Don't** style Personal, ready, working, or loading by color alone.
- **Don't** describe the renderer as a mobile app or add a mobile target to this desktop design system.
