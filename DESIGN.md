---
name: RepoBell
description: A quiet tray window that says what changed in your watched repos and lets you update safely.
colors:
  accent: "#1f5fd1"
  accent-deep: "#174aa6"
  accent-wash: "#f0f5fd"
  ink: "#1b1b1f"
  ink-soft: "#3d3d45"
  ink-muted: "#5b5b66"
  canvas: "#f6f6f4"
  surface: "#ffffff"
  header-band: "#ebebe8"
  chip-grey: "#eeeeec"
  hairline: "#e3e3e0"
  border-strong: "#c9c9c4"
  window-edge: "#d5d5d0"
  danger: "#a12a20"
  danger-ink: "#6f1d16"
  danger-wash: "#fbe7e5"
  danger-edge: "#e8b9b4"
  caution-ink: "#84470c"
  caution-wash: "#fceedd"
  success-ink: "#1e4d2b"
  success-edge: "#9cc4a6"
typography:
  title:
    fontFamily: "Ubuntu, system-ui, -apple-system, sans-serif"
    fontSize: "15px"
    fontWeight: 700
  headline:
    fontFamily: "Ubuntu, system-ui, -apple-system, sans-serif"
    fontSize: "18px"
    fontWeight: 700
  body:
    fontFamily: "Ubuntu, system-ui, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.4
  label:
    fontFamily: "Ubuntu, system-ui, -apple-system, sans-serif"
    fontSize: "13px"
    fontWeight: 500
  caption:
    fontFamily: "Ubuntu, system-ui, -apple-system, sans-serif"
    fontSize: "12px"
    fontWeight: 500
  mono:
    fontFamily: "Ubuntu Mono, ui-monospace, monospace"
    fontSize: "12px"
    fontWeight: 400
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  window: "12px"
spacing:
  xs: "6px"
  sm: "8px"
  md: "10px"
  lg: "12px"
  xl: "16px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "0 16px"
    height: "38px"
  button-primary-hover:
    backgroundColor: "{colors.accent-deep}"
  button-outline:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.accent}"
    rounded: "{rounded.md}"
    padding: "0 16px"
    height: "40px"
  button-outline-hover:
    backgroundColor: "{colors.accent-wash}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0 16px"
    height: "40px"
  button-text:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink-soft}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "38px"
  button-undo:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.success-ink}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "40px"
  button-text-blue:
    backgroundColor: "transparent"
    textColor: "{colors.accent}"
    rounded: "{rounded.md}"
    padding: "0 8px"
    height: "32px"
  button-text-blue-hover:
    backgroundColor: "{colors.accent-wash}"
  repo-row:
    backgroundColor: "{colors.surface}"
    padding: "9px 12px"
  check-strip:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.label}"
    padding: "8px 16px 9px"
  badge-new:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    typography: "{typography.caption}"
    rounded: "{rounded.sm}"
    padding: "3px 9px"
  badge-error:
    backgroundColor: "{colors.danger-wash}"
    textColor: "{colors.danger}"
    typography: "{typography.caption}"
    rounded: "{rounded.sm}"
    padding: "3px 9px"
  chip-grey:
    backgroundColor: "{colors.chip-grey}"
    textColor: "{colors.ink-soft}"
    typography: "{typography.caption}"
    rounded: "{rounded.sm}"
    padding: "4px 10px"
  chip-caution:
    backgroundColor: "{colors.caution-wash}"
    textColor: "{colors.caution-ink}"
    typography: "{typography.caption}"
    rounded: "{rounded.sm}"
    padding: "4px 10px"
  banner-error:
    backgroundColor: "{colors.danger-wash}"
    textColor: "{colors.danger-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "10px 12px"
  select-input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0 12px"
    height: "44px"
---

# Design System: RepoBell

## Overview

**Creative North Star: "The Status Light"**

RepoBell is a small panel that you glance at. Like a status light on a desk, it should be calm until something needs you. The window is plain, flat and GNOME-native. It uses one blue, white lists on a warm grey ground, and thin borders. Nothing decorates. Everything that is not news steps back into grey.

The feel is quiet, plain and utilitarian. Text is short. Controls are large enough to hit with a mouse in a hurry. Density is medium: 420 by 640 pixels holds a few grouped lists of rows, not a dashboard.

The light theme is the only theme today. The code shows no dark mode.

**Key Characteristics:**
- Flat surfaces separated by 1px borders, not shadows.
- One accent blue, used for news, primary actions and focus.
- Warm neutral greys, never pure grey or black.
- Red, amber and green appear only as status, always as a pale wash with dark text.
- Ubuntu type at 12 to 18px, weights 400, 500 and 700.
- Large hit areas: 36 to 44px high controls. Row buttons are 32px because the whole row is also a hit area.

## Colors

A warm grey ground with white lists, one blue, and three pale status washes.

### Primary
- **Signal Blue** (#1f5fd1): The only accent. The "new" badge, the Update button, links, the focus ring, the selected switch and the inset ring that flashes a row.
- **Signal Blue Deep** (#174aa6): Hover state of the filled blue button only.
- **Blue Wash** (#f0f5fd): Hover background of outline and open-in-editor buttons.

### Neutral
- **Ink** (#1b1b1f): Body text and headings.
- **Soft Ink** (#3d3d45): Secondary labels, text buttons and chip text.
- **Muted Ink** (#5b5b66): Hints, field labels, captions and loading text. The most used grey.
- **Warm Canvas** (#f6f6f4): The window ground and inset boxes inside rows.
- **Paper White** (#ffffff): Group lists, footer, menus, inputs and outline buttons.
- **Header Band** (#ebebe8): The header strip.
- **Chip Grey** (#eeeeec): Branch and neutral chips, text button hover.
- **Hairline** (#e3e3e0): List borders, row dividers and the footer divider.
- **Strong Border** (#c9c9c4): Input and secondary button borders.
- **Window Edge** (#d5d5d0): The outer border of the window.

### Status
- **Alert Red** (#a12a20) on **Red Wash** (#fbe7e5) with **Red Edge** (#e8b9b4) and **Red Ink** (#6f1d16): Errors, banners and the unsaved-work alert.
- **Caution Amber** text (#84470c) on **Amber Wash** (#fceedd): Warning chips and notices.
- **Undo Green** (#1e4d2b) with **Green Edge** (#9cc4a6): The Undo button.

### Named Rules
**The One Blue Rule.** Blue means "this is new or this is the action." Never use it for decoration or for plain status.

**The Pale Wash Rule.** Red, amber and green are never solid fills. They are a pale tint, a border and dark text. The only solid fill is the blue "new" badge.

## Typography

**Display Font:** none. The app has no display face.
**Body Font:** Ubuntu (with system-ui, -apple-system, sans-serif)
**Label/Mono Font:** Ubuntu Mono (with ui-monospace, monospace)

**Character:** One family, the platform's own. Hierarchy comes from size and weight, not from a second typeface.

### Hierarchy
- **Headline** (700, 18px): Welcome and empty-state headings.
- **Title** (700, 15px): Header title. Repo names on rows are 14px, 700.
- **Body** (400 to 500, 14px, 1.4): Buttons, banners, row text. The workhorse size.
- **Label** (500, 13px): Hints, field labels, small buttons and links.
- **Caption** (500, 12px): Badges, chips and field hints.
- **Mono** (400, 12 to 13px): File paths, folder paths and Git details (stash names, commands). Never for prose.

### Named Rules
**The Plain Size Rule.** Use only 12, 13, 14, 15 and 18px. A new size needs a reason that is not taste.

## Layout

A single column inside a fixed 420 by 640 window. The main body scrolls and has 16px padding. Group lists stack with a 12px gap. Inside a row, items use a 10px gap, and tight groups use 6 or 8px. Spacing steps in use: 6, 8, 10, 12, 16. Buttons sit in flex rows with their text vertically centered. There are no breakpoints, because the window size is fixed. Hit targets are 36px minimum, and 40 to 44px for main actions and form controls. Row buttons are the exception at 32px, because the whole row is also a hit area.

### Group lists and rows
- **Group list:** Paper White, 10px radius, 1px Hairline border, Hairline dividers between rows. One list per group.
- **Section label:** 12px, 700, uppercase, Soft Ink, with the count in Muted Ink (500).
- **Row:** about 9px 12px padding, two lines. Click anywhere on it to open it. A branch name that is too long is cut with an ellipsis and shows in full on hover. The main status and flags are never cut.
- **Feed titles:** one line each, cut with an ellipsis, full title on hover. The bullet stays.
- **Row states:** hover (lines 1 and 2 only) and open use #fbfbf9. A notification jump flashes an inset 2px Signal Blue ring on #f4f8fe. An open alert shows an inset 1.5px red (#c4372b) ring.

### Status strip
Sits between the header band and the scrolling body while a check runs.
- **Strip:** Paper White, 1px Hairline bottom border, 8px 16px 9px padding.
- **Line:** 13px. "Checking" in Soft Ink, the repo name in Ink (500) and cut with an ellipsis, the count in Soft Ink with tabular numbers.
- **Bar:** 4px high, Chip Grey track, Muted Ink fill, 2px radius. Never blue, because progress is not news.

## Elevation & Depth

The system is flat. Depth comes from 1px borders and the step between warm canvas and white. A single soft shadow lifts the whole window off the desktop. Other shadows are small functional exceptions.

### Shadow Vocabulary
- **Window lift** (`box-shadow: 0 10px 30px rgba(0, 0, 0, 0.15)`): The app root only.
- **Menu lift** (`box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12)`): The floating menu in Settings.
- **Flash ring** (`box-shadow: inset 0 0 0 2px #1f5fd1`): Highlighted row after a notification, on a #f4f8fe fill.
- **Switch thumb** (`box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25)`): The toggle knob.

### Named Rules
**The Flat List Rule.** Lists and rows never carry a shadow at rest. Borders do the separating.

## Shapes

Softly rounded rectangles. The window is 12px. Group lists and menus are 10px. Buttons, inputs, banners and inset boxes are 8px. Badges and chips are 6px. Switches are full pills and dots are circles. Borders are 1px. Nothing is clipped or cut at an angle.

## Components

### Buttons
- **Shape:** 8px radius, 36 to 40px high, 14px text. Row buttons are 32px high with 13px text.
- **Primary:** Filled Signal Blue, white text, 500 weight, 0 16px padding (38px high). Hover goes to Signal Blue Deep.
- **Outline:** White fill, blue 1px border and text. The Update button on a row (32px high). Hover adds Blue Wash.
- **Update main:** No fill, Signal Blue text, 32px high, 0 8px padding. Hover adds Blue Wash. A quiet text button, so a blue outline stays a sign of team news. The "Mark all as seen" button on the News label is the same style with a 6px radius.
- **Secondary:** White fill, Strong Border, Ink text, 40px high.
- **Text:** No fill, Soft Ink text, Chip Grey on hover. Used for cancel and dismiss.
- **Undo:** White fill, Green Edge border, green text. Disabled goes flat grey (#f3f3f1 fill).
- **Disabled:** 60% opacity, default cursor.
- **Focus:** 2px Signal Blue outline with 2px offset, on every focusable element.

### Containers
- **Corner Style:** 10px.
- **Background:** Paper White on Warm Canvas.
- **Border:** 1px Hairline.
- **Inset box:** Warm Canvas fill, Hairline border, 8px radius, 12px padding. Holds the update prompt and previews.

### Badges and Chips
- **Badge:** 12px, 500, 6px radius, 3px 9px. Solid blue for "new", red wash for errors.
- **Chip:** 12px, 500, 6px radius, 4px 10px. Grey for branch and neutral status, amber wash for caution. Not interactive; the cursor stays default and a hover explains the term.

### Inputs / Fields
- **Style:** White fill, 1px Strong Border, 8px radius, 44px high, 12px side padding.
- **Label:** 13px Muted Ink above. Hint is 12px Muted Ink.
- **Switch:** 44 by 26px pill, grey (#8a8a94) off, Signal Blue on, white thumb that slides over 0.15s.

### Banner
Red wash, Red Edge border, Red Ink 14px text, 8px radius, with an underlined text-button dismiss.

### Navigation
A header band (#ebebe8) with a 1px bottom border, a centered 15px bold title and 8px-radius icon buttons that darken on hover (#e0e0dc). The footer is white, 52px high, with a Hairline top border and the same style of ghost buttons. There are no tabs.

### Motion
Short and functional. State changes ease over 0.15s, the row highlight over 0.3s, and spinners turn at 1s linear. The status strip bar eases its width over 0.15s, and not at all under reduced motion. Reduced-motion media queries exist in the app, the rows, Settings and Welcome.

## Do's and Don'ts

### Do:
- **Do** keep the window flat: white lists on Warm Canvas with 1px borders.
- **Do** reserve Signal Blue (#1f5fd1) for news, the main action and focus.
- **Do** make every control at least 36px high, and 40px or more for actions. Row buttons are the exception at 32px.
- **Do** show status as pale wash, border and dark text.
- **Do** keep the 2px blue focus outline on all focusable elements.
- **Do** honor `prefers-reduced-motion` for every animation.
- **Do** use Ubuntu with the system fallbacks.

### Don't:
- **Don't** add a second accent color or a solid red, amber or green fill.
- **Don't** add shadows to lists, rows or buttons.
- **Don't** introduce a new radius or font size outside the scales above.
- **Don't** use pure black or pure grey. Use the warm Ink and Canvas values.
- **Don't** show commit SHAs or Git jargon beyond the short status labels.
