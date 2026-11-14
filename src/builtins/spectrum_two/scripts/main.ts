// Spectrum 2 starter script.
//
// Every bare or subpath specifier below names a package listed in the project's
// package.json (docs/design-systems/spectrum-adapter.md, Facet 7). The project's
// package manager installs those packages into node_modules, and the native build
// resolves the imports from there and bundles them into dist/ as local files. The
// page loads no module from a CDN or any other remote host. Bare specifiers belong
// in this script, not in the page's inline script.

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
