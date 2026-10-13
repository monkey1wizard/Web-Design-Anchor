// Web Design Anchor Showcase: page script.
//
// Registers the Spectrum 2 theme provider and drives the color mode switch.
// The inline <head> script restores a stored light or dark choice on
// <html data-theme> before first paint. This module keeps <sp-theme color>,
// the radios, and storage in sync with that attribute.

import "@spectrum-web-components/theme/sp-theme.js";
import "@spectrum-web-components/theme/spectrum-two/scale-medium.js";
import "@spectrum-web-components/theme/spectrum-two/theme-light.js";
import "@spectrum-web-components/theme/spectrum-two/theme-dark.js";

type ColorMode = "light" | "dark" | "system";

const STORAGE_KEY = "wda-showcase-color-mode";
const root = document.documentElement;
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function currentMode(): ColorMode {
  const theme = root.getAttribute("data-theme");
  return theme === "light" || theme === "dark" ? theme : "system";
}

function storeMode(mode: ColorMode): void {
  try {
    if (mode === "system") {
      localStorage.removeItem(STORAGE_KEY);
    } else {
      localStorage.setItem(STORAGE_KEY, mode);
    }
  } catch {
    // Storage is blocked: the choice lasts for this page view only.
  }
}

function syncSpectrumColor(): void {
  const mode = currentMode();
  const color = mode === "system" ? (darkQuery.matches ? "dark" : "light") : mode;
  document.querySelector("sp-theme")?.setAttribute("color", color);
}

function applyMode(mode: ColorMode): void {
  if (mode === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", mode);
  }
  storeMode(mode);
  syncSpectrumColor();
}

const initialMode = currentMode();
for (const radio of document.querySelectorAll<HTMLInputElement>('input[name="color-mode"]')) {
  radio.checked = radio.value === initialMode;
  radio.addEventListener("change", () => {
    if (radio.checked) {
      applyMode(radio.value as ColorMode);
    }
  });
}

// Escape hides the visible tooltip of the hovered or focused segment.
const tipOptions = document.querySelectorAll<HTMLElement>(".color-mode__option");
for (const option of tipOptions) {
  const reset = () => option.classList.remove("is-tip-dismissed");
  option.addEventListener("pointerleave", reset);
  option.addEventListener("focusout", reset);
}
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  for (const option of tipOptions) {
    if (option.matches(":hover, :has(:focus-visible)")) {
      option.classList.add("is-tip-dismissed");
    }
  }
});

darkQuery.addEventListener("change", () => {
  if (currentMode() === "system") {
    syncSpectrumColor();
  }
});
