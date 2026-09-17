---
title: Design System & Visual Review Criteria
status: active
updated: 2026-08-27
---

# Design System & Visual Review Criteria

This document records the project-specific visual-review criteria instantiated for this project.

- **Design System**: {{DESIGN_SYSTEM_NAME}}
- **Accessibility Baseline**: {{ACCESSIBILITY_BASELINE}}

## Attribution

The design token values referenced throughout this document originate from Adobe Spectrum and are reused under the Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0). The values have been modified from Adobe Spectrum's upstream defaults for this project.

## 1. Visual Hierarchy

{{#if_design_system "WDA Minimal"}}
- Visual hierarchy in WDA Minimal must strictly adhere to a 2D flat layout model with at most 3 distinct visual tiers, eliminating elevation shadows, z-index depth layering, and card container wrappers.
- Heading hierarchy (`<h1>` through `<h3>`) utilizes WDA Minimal's font-size scale (`--font-size-sm` through `--font-size-2xl`, a 5-step span) and weight contrast (400 vs 700) to signal section importance without visual elevation.
- Section boundaries and tier separation rely exclusively on subtle 1px border lines (`--color-border-default`) and spatial whitespace padding (`--spacing-md`, `--spacing-lg`).
- Tier contrast and section spacing must satisfy {{ACCESSIBILITY_BASELINE}} structural hierarchy requirements for assistive navigation without visual depth tokens.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Visual hierarchy in Spectrum 2 follows a reduced-elevation surface model, replacing Classic Spectrum's heavy layered shadows with flatter surfaces distinguished by color fills and 1px hairline borders.
- Heading hierarchy (`<h1>` through `<h3>`) utilizes Spectrum 2's refreshed Adobe Clean type scale (`Spectrum2-Heading-XL` through `Spectrum2-Heading-S`) with revised line heights tuned for the unified Spectrum 2 visual language.
- Surface separation relies primarily on Spectrum 2's rounded-corner container tokens (`--spectrum2-corner-radius-100` through `--spectrum2-corner-radius-400`) and softened border colors rather than drop shadows.
- Tier contrast and rounded-surface boundaries must satisfy {{ACCESSIBILITY_BASELINE}} structural hierarchy and luminosity contrast thresholds across the unified Spectrum 2 light and dark themes.
{{/if_design_system}}

## 2. Typographic Rhythm

{{#if_design_system "WDA Minimal"}}
- Typographic scale uses system UI fonts (`system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`) with a strict 1.5 line-height ratio (`--line-height-base: 1.5`) and 1.25 heading ratio (`--line-height-tight: 1.25`).
- Vertical rhythm across headings, lead text, and body paragraphs adheres to rem-based spacing tokens (`--spacing-xs` to `--spacing-xl`) without inline overrides or arbitrary pixel offsets.
- Text sizing and line spacing remain readable and scalable under {{ACCESSIBILITY_BASELINE}} 200% text zoom and reflow requirements without horizontal overflow or clipping.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Typographic rhythm conforms to Spectrum 2's refreshed Adobe Clean type scale, using strict 1.3 heading line-height ratios and 1.5 body line-height ratios defined by Spectrum 2 typography tokens.
- Vertical rhythm aligns to Spectrum 2's 8px base grid (widened from Classic Spectrum's 4px grid) with multi-device typography scaling rules across web, desktop, and mobile viewports.
- Variable weight axes and font optical sizing conform to {{ACCESSIBILITY_BASELINE}} minimum legibility, letter-spacing, and line-length (max 80 characters per line) standards.
{{/if_design_system}}

## 3. Spacing & Density

{{#if_design_system "WDA Minimal"}}
- Spatial layout and container padding are governed strictly by WDA Minimal's 6-token DTCG spacing scale (`--spacing-xs`: 0.25rem, `--spacing-sm`: 0.5rem, `--spacing-md`: 1rem, `--spacing-lg`: 1.5rem, `--spacing-xl`: 2rem, `--spacing-2xl`: 3rem).
- Maintains a relaxed spatial density profile for interactive controls and text containers to maximize legibility and touch comfort without compact density options.
- Container padding and item margins must satisfy {{ACCESSIBILITY_BASELINE}} minimum touch target (24x24px / 44x44px) boundaries without hardcoded pixel offsets.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Spatial layout and density follow Spectrum 2's unified density model, consolidating Classic Spectrum's `spacious`, `regular`, and `compact` tiers into two selectable density options exposed via root CSS context rules.
- Grid spacing follows Spectrum 2's 8px spatial grid system with larger default component padding for touch and pointer input modes.
- Interactive spacing and padding rules satisfy {{ACCESSIBILITY_BASELINE}} touch target spacing and focus indicator clearance guidelines across both density modes.
{{/if_design_system}}

## 4. CTA Emphasis

{{#if_design_system "WDA Minimal"}}
- Primary actions utilize high-contrast flat WDA Minimal interactive tokens (`--color-brand-primary: #2563eb`, `--color-text-on-brand: #ffffff`) with 1px solid borders and subtle hover background transitions (`--color-brand-hover: #1d4ed8`).
- Secondary actions use outline styles (`background: transparent`, `border: 1px solid --color-border-default`) and tertiary actions use plain text links without background containers.
- Button contrast ratios (minimum 4.5:1 against page surface) and focus ring outline (`2px solid --color-border-focus`) satisfy {{ACCESSIBILITY_BASELINE}} interactive control accessibility rules.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Primary actions utilize Spectrum 2 Accent Button components (`--spectrum2-accent-color-900`) with softened rounded corners, refined press states, and brand accent fill.
- Secondary and quiet buttons follow Spectrum 2's consolidated Action Button contract, replacing Classic Spectrum's separate quiet-button variant with a single configurable emphasis level.
- Focus indicators use Spectrum 2's single thickened focus ring (`--spectrum2-focus-indicator-color`), replacing Classic Spectrum's double-ring outline, and satisfy {{ACCESSIBILITY_BASELINE}} enhanced focus contrast and state transition requirements.
{{/if_design_system}}

## 5. Compositional Balance

{{#if_design_system "WDA Minimal"}}
- Page layout relies on native CSS Grid and Flexbox with a single centered max-width container (fixed at `1200px`, not a design token) and 1rem side gutting.
- Structural divisions use subtle 1px border lines (`--color-border-default: #e2e8f0`) rather than drop shadows, card wrappers, or decorative background shapes.
- Document layout follows linear single-column or symmetrical two-column arrangements that maintain logical DOM flow under {{ACCESSIBILITY_BASELINE}} screen reader navigation.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Page composition utilizes Spectrum 2 Grid and Flex layout primitives, supporting multi-column application shells, persistent side navigation panels, and split-pane viewports.
- Component containers use Spectrum 2 Card and Well surface containers with softened, more pronounced corner rounding (`--spectrum2-corner-radius-200`) and hairline borders in place of Classic Spectrum's heavier drop shadows.
- Layout regions follow ARIA landmark roles (`main`, `navigation`, `complementary`) and satisfy {{ACCESSIBILITY_BASELINE}} landmark navigation and reflow rules.
{{/if_design_system}}

## 6. Image Cropping & Media Handling

{{#if_design_system "WDA Minimal"}}
- Media elements scale fluidly within native CSS container constraints (`max-width: 100%`, `height: auto`) using `object-fit: cover` for aspect-ratio preservation without Javascript cropping scripts.
- Decorative SVG icons and images are embedded inline with `aria-hidden="true"`, while content images require explicit descriptive `alt` text.
- Non-text media alternatives and responsive image scaling satisfy {{ACCESSIBILITY_BASELINE}} non-text content and high-contrast display mode rules.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Media handling conforms to Spectrum 2 Asset and Thumbnail component specifications, supporting aspect-ratio locked frames (16:9, 4:3, 1:1) and refreshed loading skeletons.
- Image cropping boxes utilize Spectrum 2 Asset Cover tokens with the system's softened corner-radius scale and focal-point alignment attributes.
- Informational imagery includes structured captions and text descriptions compliant with {{ACCESSIBILITY_BASELINE}} accessible name and non-text content criteria.
{{/if_design_system}}

## 7. Responsive Widths

{{#if_design_system "WDA Minimal"}}
- Responsive adaptations target three breakpoint tiers: Wide (>1024px, 3-column feature grid), Medium (768px - 1023px, 2-column grid), and Narrow (<767px, single-column stacked layout).
- Navigation switches from horizontal flex row on wide viewports to vertically stacked list on narrow viewports without Javascript menu drawer dependencies.
- Viewport reflow supports 320px minimum screen width without horizontal scrolling or content truncation under {{ACCESSIBILITY_BASELINE}} reflow standards.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Responsive breakpoints align with Spectrum 2 media query tokens (`S`: 600px, `M`: 960px, `L`: 1280px, `XL`: 1440px), adjusting component density and grid layout dynamically.
- Navigation adapts via Spectrum 2 Drawer and ActionBar controls, transitioning sidebars into modal overlays on compact viewports.
- Mobile reflow and dynamic font scaling conform to {{ACCESSIBILITY_BASELINE}} responsive viewport and orientation change guidelines without two-dimensional scrolling.
{{/if_design_system}}

## 8. Brand Fit & Aesthetic Alignment

{{#if_design_system "WDA Minimal"}}
- Visual presentation emphasizes unadorned, modern neutral clarity with a monochromatic slate palette (`--color-brand-primary`, `--color-background-surface`, `--color-text-primary`) and clean typographic hierarchy.
- Styling strictly avoids decorative gradients, heavy shadows, custom webfonts, and non-tokenized CSS rules.
- Minimal aesthetic integrity is maintained while satisfying {{ACCESSIBILITY_BASELINE}} contrast, legibility, and high-contrast color scheme requirements.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- Visual presentation reflects Spectrum 2's unified brand expression, consolidating Classic Spectrum and Express theme variants into a single softened, more colorful, and rounder visual language.
- Styling utilizes Spectrum 2 color themes (`light`, `dark`) and platform-matched system font stacks, dropping Classic Spectrum's separate `lightest`/`darkest` theme variants.
- Brand identity conforms to Spectrum 2 brand guidelines while satisfying all {{ACCESSIBILITY_BASELINE}} color perception and visual customization standards.
{{/if_design_system}}

## 9. Accessibility Baseline Criteria

{{#if_design_system "WDA Minimal"}}
- **Color Contrast**: All text elements achieve at least 4.5:1 contrast against `--color-background-surface`, and interactive controls achieve at least 3:1 against background colors per {{ACCESSIBILITY_BASELINE}}.
- **Focus Appearance**: Keyboard focus displays a high-contrast solid outline (`2px solid --color-border-focus`, `outline-offset: 2px`) without hiding default browser focus.
- **Touch Targets**: Interactive controls maintain a minimum target area of 24x24 pixels (or 44x44 pixels where required by {{ACCESSIBILITY_BASELINE}}).
- **Keyboard Navigation**: All interactive elements are reachable and operable using standard Tab and Shift+Tab key sequences.
{{/if_design_system}}
{{#if_design_system "Spectrum 2"}}
- **Color Contrast**: Text and UI components satisfy {{ACCESSIBILITY_BASELINE}} enhanced contrast ratios (minimum 4.5:1 normal text, 3:1 graphical objects/UI components across the unified Spectrum 2 light and dark themes).
- **Focus Appearance**: Focus indicators utilize Spectrum 2's single thickened focus ring (`--spectrum2-focus-indicator-color`), replacing Classic Spectrum's double-ring outline, with high-contrast offset against adjacent surfaces.
- **Touch Targets**: Interactive controls satisfy Spectrum 2's enlarged touch target size specifications (minimum 44x44px for touch input modes, increased from Classic Spectrum's 40x40px) per {{ACCESSIBILITY_BASELINE}}.
- **Keyboard Navigation**: Full keyboard navigation support includes ARIA composite widget key handlers (arrow keys for tabs, menus, and option groups) under {{ACCESSIBILITY_BASELINE}}.
{{/if_design_system}}
