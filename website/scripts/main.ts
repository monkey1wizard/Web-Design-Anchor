// Web Design Anchor Website: page script.
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

const STORAGE_KEY = "wda-website-color-mode";
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

// Colors cross-fade for one switch only. main.css holds the transition.
let themeFadeTimer = 0;

function applyMode(mode: ColorMode): void {
  root.classList.add("is-theme-fading");
  window.clearTimeout(themeFadeTimer);
  themeFadeTimer = window.setTimeout(() => root.classList.remove("is-theme-fading"), 260);
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

// Copy buttons on the install commands. The button is shown only when
// <html data-js> is set, so without JavaScript the command stays selectable.
const copyStatus = document.getElementById("copy-status");

function selectText(element: Element): void {
  const range = document.createRange();
  range.selectNodeContents(element);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}

for (const button of document.querySelectorAll<HTMLButtonElement>(".cmd__copy")) {
  let resetTimer = 0;
  button.addEventListener("click", async () => {
    const text = button.closest(".cmd")?.querySelector(".cmd__text");
    if (!text?.textContent) return;
    try {
      await navigator.clipboard.writeText(text.textContent.trim());
      button.classList.add("is-copied");
      if (copyStatus) copyStatus.textContent = "已複製";
      window.clearTimeout(resetTimer);
      resetTimer = window.setTimeout(() => {
        button.classList.remove("is-copied");
        if (copyStatus) copyStatus.textContent = "";
      }, 2000);
    } catch {
      // Clipboard access is blocked: select the command so it can be copied by hand.
      selectText(text);
      if (copyStatus) copyStatus.textContent = "無法自動複製，已選取指令，請手動複製";
    }
  });
}

// Replays: the dist/ tree unfolds and the build output prints, once each,
// when they first scroll into view. Content stays visible unless this script
// arms a replay, so a failed script, a browser without IntersectionObserver,
// or a reduced motion preference all leave the static page.
const REPLAY_MS = 1200;
const replays = document.querySelectorAll<HTMLElement>("[data-replay]");
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

if (!reduceMotion && "IntersectionObserver" in window && replays.length > 0) {
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      const replay = entry.target as HTMLElement;
      observer.unobserve(replay);
      replay.classList.add("is-replaying");
      window.setTimeout(() => replay.classList.remove("is-replay-armed", "is-replaying"), REPLAY_MS);
    }
  }, { threshold: 0.35 });

  for (const replay of replays) {
    const steps = replay.querySelectorAll<HTMLElement>(".dtree__row, .term__line");
    steps.forEach((step, index) => step.style.setProperty("--i", String(index)));
    replay.classList.add("is-replay-armed");
    observer.observe(replay);
  }
}
