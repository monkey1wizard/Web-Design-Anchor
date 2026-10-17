---
title: Design System & Visual Review Criteria
status: active
updated: 2026-10-13
---

# Design System & Visual Review Criteria

This document records the project-specific visual-review criteria.

- **Design System**: Spectrum 2
- **Accessibility Baseline**: WCAG 2.2 AA

## Attribution

Adobe Spectrum provides the design token values referenced throughout this document. This project reuses those values under the Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0). This project modified the values from Adobe Spectrum's upstream defaults.

This project adapted the criteria in section 10 from impeccable (https://github.com/pbakaus/impeccable), licensed under the Apache License, Version 2.0. This project selected and rewrote the criteria, and they differ from the upstream text.

## 1. Visual Hierarchy

- Visual hierarchy in Spectrum 2 follows a reduced-elevation surface model, replacing Classic Spectrum's heavy layered shadows with flatter surfaces distinguished by color fills and 1px hairline borders.
- Heading hierarchy (`<h1>` through `<h3>`) utilizes Spectrum 2's refreshed Adobe Clean type scale (`Spectrum2-Heading-XL` through `Spectrum2-Heading-S`) with revised line heights tuned for the unified Spectrum 2 visual language.
- Surface separation relies primarily on Spectrum 2's rounded-corner container tokens (`--spectrum2-corner-radius-100` through `--spectrum2-corner-radius-400`) and softened border colors rather than drop shadows.
- Tier contrast and rounded-surface boundaries must satisfy WCAG 2.2 AA structural hierarchy and luminosity contrast thresholds across the unified Spectrum 2 light and dark themes.

## 2. Typographic Rhythm

- Typographic rhythm conforms to Spectrum 2's refreshed Adobe Clean type scale, using strict 1.3 heading line-height ratios and 1.5 body line-height ratios defined by Spectrum 2 typography tokens.
- Vertical rhythm aligns to Spectrum 2's 8px base grid (widened from Classic Spectrum's 4px grid) with multi-device typography scaling rules across web, desktop, and mobile viewports.
- Variable weight axes and font optical sizing conform to WCAG 2.2 AA minimum legibility, letter-spacing, and line-length (max 80 characters per line) standards.

## 3. Spacing & Density

- Spatial layout and density follow Spectrum 2's unified density model, consolidating Classic Spectrum's `spacious`, `regular`, and `compact` tiers into two selectable density options exposed via root CSS context rules.
- Grid spacing follows Spectrum 2's 8px spatial grid system with larger default component padding for touch and pointer input modes.
- Interactive spacing and padding rules satisfy WCAG 2.2 AA touch target spacing and focus indicator clearance guidelines across both density modes.

## 4. CTA Emphasis

- Primary actions utilize Spectrum 2 Accent Button components (`--spectrum2-accent-color-900`) with softened rounded corners, refined press states, and brand accent fill.
- Secondary and quiet buttons follow Spectrum 2's consolidated Action Button contract, replacing Classic Spectrum's separate quiet-button variant with a single configurable emphasis level.
- Focus indicators use Spectrum 2's single thickened focus ring (`--spectrum2-focus-indicator-color`), replacing Classic Spectrum's double-ring outline, and satisfy WCAG 2.2 AA enhanced focus contrast and state transition requirements.

## 5. Compositional Balance

- Page composition utilizes Spectrum 2 Grid and Flex layout primitives, supporting multi-column application shells, persistent side navigation panels, and split-pane viewports.
- Component containers use Spectrum 2 Card and Well surface containers with softened, more pronounced corner rounding (`--spectrum2-corner-radius-200`) and hairline borders in place of Classic Spectrum's heavier drop shadows.
- Layout regions follow ARIA landmark roles (`main`, `navigation`, `complementary`) and satisfy WCAG 2.2 AA landmark navigation and reflow rules.

## 6. Image Cropping & Media Handling

- Media handling conforms to Spectrum 2 Asset and Thumbnail component specifications, supporting aspect-ratio locked frames (16:9, 4:3, 1:1) and refreshed loading skeletons.
- Image cropping boxes utilize Spectrum 2 Asset Cover tokens with the system's softened corner-radius scale and focal-point alignment attributes.
- Informational imagery includes structured captions and text descriptions compliant with WCAG 2.2 AA accessible name and non-text content criteria.

## 7. Responsive Widths

- Responsive breakpoints align with Spectrum 2 media query tokens (`S`: 600px, `M`: 960px, `L`: 1280px, `XL`: 1440px), adjusting component density and grid layout dynamically.
- Navigation adapts via Spectrum 2 Drawer and ActionBar controls, transitioning sidebars into modal overlays on compact viewports.
- Mobile reflow and dynamic font scaling conform to WCAG 2.2 AA responsive viewport and orientation change guidelines without two-dimensional scrolling.

## 8. Brand Fit & Aesthetic Alignment

- Visual presentation reflects Spectrum 2's unified brand expression, consolidating Classic Spectrum and Express theme variants into a single softened, more colorful, and rounder visual language.
- Styling utilizes Spectrum 2 color themes (`light`, `dark`) and platform-matched system font stacks, dropping Classic Spectrum's separate `lightest`/`darkest` theme variants.
- Brand identity conforms to Spectrum 2 brand guidelines while satisfying all WCAG 2.2 AA color perception and visual customization standards.

## 9. Accessibility Baseline Criteria

- **Color Contrast**: Text and UI components satisfy WCAG 2.2 AA enhanced contrast ratios (minimum 4.5:1 normal text, 3:1 graphical objects/UI components across the unified Spectrum 2 light and dark themes).
- **Focus Appearance**: Focus indicators utilize Spectrum 2's single thickened focus ring (`--spectrum2-focus-indicator-color`), replacing Classic Spectrum's double-ring outline, with high-contrast offset against adjacent surfaces.
- **Touch Targets**: Interactive controls satisfy Spectrum 2's enlarged touch target size specifications (minimum 44x44px for touch input modes, increased from Classic Spectrum's 40x40px) per WCAG 2.2 AA.
- **Keyboard Navigation**: Full keyboard navigation support includes ARIA composite widget key handlers (arrow keys for tabs, menus, and option groups) under WCAG 2.2 AA.

## 10. Craft Floor

These criteria apply to every design system. This project adapted them from impeccable; see Attribution. The Designer may change or remove any criterion for this project. Record each change in this section.

- **Spacing rhythm**: Related elements sit closer together than unrelated groups. The space above a heading is larger than the space below it.
- **Real content**: Review every page with its real copy at every responsive breakpoint. Fix any text that overflows or is clipped.
- **Interactive states**: Every interactive control defines hover and focus states, and a disabled state where the control can be disabled. Form fields define an error state.
- **Browser surfaces**: Set link underline offset (`text-underline-offset`) and numerals in data tables (`font-variant-numeric: tabular-nums`) explicitly instead of leaving them at browser defaults.
- **Action copy**: Buttons name the action they perform. Error messages state the problem and how to recover.
- **Text on colored surfaces**: Secondary text on a colored surface uses a token paired with that surface, not a neutral gray, and still meets the contrast in section 9. If no paired token exists, ask the Designer before adding one.
- **No gradient text**: Emphasis comes from weight or size.
- **Cards and stripes**: Cards do not nest inside cards. Cards, list items, callouts, and alerts do not use a colored border on only the left or right side that is wider than 1px.
- **Icons**: Icons are inline SVG or the selected design system's icon set, with consistent stroke and weight. Emoji and Unicode symbols do not stand in for icons.
- **Section numbers**: Sections are not numbered 01 / 02 / 03 unless the sequence carries information the reader needs.
- **Hero**: The hero does not use the "big number, small label, supporting stats" template.

## 11. Project Direction

This section records the direction chosen for this website on 2026-10-14. It replaces the four-step strip direction of 2026-10-13. Sections 1 to 10 stay the review criteria. This section says what this page is trying to be.

- **Thesis**: The page answers the five questions a first-time visitor asks, and each answer shows real evidence: an example exchange, the project folders the Designer and the AI maintain, the `dist/` folder that `wda build` creates with measured file sizes, the real `wda build` output, and the install routes. The project chapter states that the design is maintained through `docs/design.md`, and shows `dist/` only as a collapsed folder. The next chapter expands `dist/`. It refuses the template of a headline, a button pair, and a row of equal icon cards. It also refuses telling the same story twice and repeating one section shell.
- **Own world**: Spectrum 2 tokens and the project's token font stack. A light gray page. Each question is a section with a large heading, a one-line answer, and its evidence. File names and output are set in monospace without a gray chip. The build output takes the opposite mode's surface in both light and dark. The primary brand color marks the question links. The project chapter and the `dist/` chapter each use a folder tree. Folders use a golden icon, and HTML, CSS, and JavaScript files use orange, blue, and yellow icons. These icon colors come from the `color.filetype` tokens and stay the same in both modes. Blue link text marks links. Motion follows the Motion entry below.
- **Story**: The visitor finds their own question, sees evidence instead of claims, comes to believe nothing locks them in, and installs WDA.
- **First viewport**: A top bar at the end of the container holds the language switch, three color-mode icons, and the GitHub link. The `<h1>` reads 「Web Design Anchor」 at the start edge with a one-line subtitle. At 1280px and wider, the five questions sit beside the title, aligned to its top. Below 1280px, they stack under the subtitle. Each question link starts with a chevron in the primary brand color and has no divider line.
- **Layout by width**: Below 1280px, every section stacks vertically. At 1280px and wider, each question puts its heading and answer beside its evidence.
- **Motion**: Changed on 2026-10-17. Before that date, this section allowed no entrance animation. Motion confirms an action or replays evidence. It never decorates. Two replays are allowed, and each plays once when it first scrolls into view: the `dist/` tree unfolds, and the `wda build` output prints line by line. Feedback motion covers the copy button, the question-link chevron, a brand-color underline, as wide as the heading text, on the heading a question link lands on, and a color cross-fade when the color mode changes. No other section, the first viewport, or the page background may use an entrance animation. Content is visible without JavaScript, and a replay hides content only after the script arms it. Motion uses only CSS transitions, keyframes, and IntersectionObserver, which are Baseline Widely Available features, and adds no library. Movement uses only transform, opacity, and clip-path, plus the stroke of the copy button's check mark, so no animation changes layout. With reduced motion, the replays and movement stop, and only color and opacity changes remain.
- **Tone**: Plain and factual. Show real output and real file names, and let them make the case.
- **Anti-references**: A template look, such as eyebrow labels, equal card grids, tag pills, gradient panels, floating chips, and abstract wireframe art. Overclaiming, meaning any statement that `README.md` or the WDA documentation does not support.
