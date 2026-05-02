# Color Palette & Brand Style

---

## Shape Colors (Semantic)

Colors encode meaning, not decoration. Each semantic purpose has a fill/stroke
pair.

| Semantic Purpose  | Fill        | Stroke                        |
| ----------------- | ----------- | ----------------------------- |
| Primary/Neutral   | `#81a2be`   | `#c5c8c6`                     |
| Secondary         | `#8abeb7`   | `#c5c8c6`                     |
| Tertiary          | `#214283`   | `#81a2be`                     |
| Start/Trigger     | `#ea9e46`   | `#c5c8c6`                     |
| End/Success       | `#b5bd68`   | `#c5c8c6`                     |
| Warning/Reset     | `#cc6666`   | `#c5c8c6`                     |
| Decision          | `#f0c674`   | `#c5c8c6`                     |
| AI/LLM            | `#b294bb`   | `#c5c8c6`                     |
| Inactive/Disabled | `#454547`   | `#666666` (use dashed stroke) |
| Error             | `#cc6666`   | `#fdff50`                     |
| Grouping/Boundary | transparent | `#666666` (use dashed stroke) |

**Rule**: Always pair a darker stroke with a lighter fill for contrast.

**Corners**: All rectangles use square corners (`"roundness": null`). Do not use
rounded corners.

**Grouping/Boundary** wraps related subcomponents in a dashed rectangle with no
fill — used for module boundaries, trust/network zones, transaction scopes, or
"only when X" conditional regions. Place the group label in the top-left corner
using `#969896` body text. Solid shapes inside still use their own semantic
colors; the dashed wrapper does not compete visually.

---

## Text Colors (Hierarchy)

Use color on free-floating text to create visual hierarchy without containers.

| Level          | Color     | Use For                             |
| -------------- | --------- | ----------------------------------- |
| Title          | `#ced0d6` | Section headings, major labels      |
| Subtitle       | `#c5c8c6` | Subheadings, secondary labels       |
| Body/Detail    | `#969896` | Descriptions, annotations, metadata |
| On light fills | `#1d1f21` | Text inside light/yellow shapes     |
| On dark fills  | `#ffffff` | Text inside saturated shapes        |

---

## Evidence Artifact Colors

Used for code snippets, data examples, and other concrete evidence inside
technical diagrams.

| Artifact          | Background | Text Color                            |
| ----------------- | ---------- | ------------------------------------- |
| Code snippet      | `#1d1f21`  | Syntax-colored (language-appropriate) |
| JSON/data example | `#1d1f21`  | `#b5bd68` (terminal green)            |

Add a `#454547` stroke around evidence artifacts so they separate from the
canvas background.

---

## Default Stroke & Line Colors

| Element                                       | Color                                                         |
| --------------------------------------------- | ------------------------------------------------------------- |
| Arrows                                        | Use the stroke color of the source element's semantic purpose |
| Structural lines (dividers, trees, timelines) | Foreground (`#c5c8c6`) or muted (`#666666`)                   |
| Marker dots (fill + stroke)                   | Primary fill (`#81a2be`)                                      |

---

## Background

| Property          | Value     |
| ----------------- | --------- |
| Canvas background | `#1d1f21` |
