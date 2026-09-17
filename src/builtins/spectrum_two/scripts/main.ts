// Spectrum 2 starter script.
//
// Every non-relative import below comes from the corrected paper adapter's
// direct-dependency set (docs/design-systems/spectrum-paper-adapter.md,
// Facet 5): theme, button, textfield, picker, card, dialog. Bare specifiers
// must never appear in the page's inline script — discover_script_entries
// (src/build/scripts.rs:397-421) only lock-checks, type-checks, and bundles
// page-referenced scripts/**/*.ts entries such as this one.

// Theme provider plus the Spectrum 2 scale and color modules the page's
// <sp-theme system="spectrum-two"> scope resolves against.
import "@spectrum-web-components/theme/sp-theme.js";
import "@spectrum-web-components/theme/spectrum-two/scale-medium.js";
import "@spectrum-web-components/theme/spectrum-two/scale-large.js";
import "@spectrum-web-components/theme/spectrum-two/theme-light.js";
import "@spectrum-web-components/theme/spectrum-two/theme-dark.js";

// Components the starter page renders.
import "@spectrum-web-components/button/sp-button.js";
import "@spectrum-web-components/textfield/sp-textfield.js";
import "@spectrum-web-components/picker/sp-picker.js";
import "@spectrum-web-components/card/sp-card.js";
import "@spectrum-web-components/dialog/sp-dialog.js";
import "@spectrum-web-components/dialog/sp-dialog-wrapper.js";

const trigger = document.getElementById("open-dialog-trigger");
const dialog = document.getElementById("starter-dialog") as
  | (HTMLElement & { open: boolean })
  | null;

trigger?.addEventListener("click", () => {
  if (dialog) {
    dialog.open = true;
  }
});
