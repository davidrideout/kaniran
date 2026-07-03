"use strict";

// Interactivity layer only — the server renders all content. This script
// never builds markup from data: it re-submits the form over fetch and
// swaps server-rendered regions, clones pre-rendered tooltip nodes, and
// flips display classes on <body>. All listeners sit on stable ancestors,
// so swapped-in content needs no rebinding.

const form = document.getElementById("analyzeForm");
const textInput = document.getElementById("textInput");
const analyzeBtn = document.getElementById("analyzeBtn");
const statusEl = document.getElementById("status");
const output = document.getElementById("output");
const stats = document.getElementById("stats");
const tooltip = document.getElementById("tooltip");
const displayBtn = document.getElementById("displayBtn");
const displayMenu = document.getElementById("displayMenu");

// ---- re-analyze: fetch the same page, swap the rendered regions ------------

async function runAnalyze() {
  if (!textInput.value.trim()) return;
  statusEl.textContent = "analyzing…";
  analyzeBtn.disabled = true;
  try {
    const response = await fetch(form.action, {
      method: "POST",
      body: new URLSearchParams(new FormData(form)),
    });
    if (!response.ok) {
      throw new Error((await response.text()) || `HTTP ${response.status}`);
    }
    const page = new DOMParser().parseFromString(await response.text(), "text/html");
    output.replaceChildren(...page.getElementById("output").childNodes);
    const newStats = page.getElementById("stats");
    stats.replaceChildren(...newStats.childNodes);
    statusEl.textContent = parseStatus(newStats);
  } catch (error) {
    statusEl.textContent = `error: ${error.message}`;
  } finally {
    analyzeBtn.disabled = false;
  }
}

// "N words parsed in X ms" from the data attributes a #stats region carries —
// the server-side segmentation time, not the request round-trip.
function parseStatus(statsEl) {
  return `${statsEl.dataset.words} words parsed in ${statsEl.dataset.parseMs} ms`;
}

// The initial page is server-parsed too; show its numbers right away.
statusEl.textContent = parseStatus(stats);

form.addEventListener("submit", (event) => {
  event.preventDefault();
  runAnalyze();
});

textInput.addEventListener("keydown", (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    event.preventDefault();
    runAnalyze();
  }
});

// ---- tooltip: clone the word's pre-rendered <template class="tt"> ----------

output.addEventListener("click", (event) => {
  const word = event.target.closest(".word");
  if (!word) return;
  event.stopPropagation();
  const body = word.querySelector("template.tt");
  if (!body) return;
  tooltip.replaceChildren(body.content.cloneNode(true));
  tooltip.hidden = false;
  positionTooltip(event);
});

function positionTooltip(event) {
  const margin = 12;
  const rect = tooltip.getBoundingClientRect();
  let left = event.clientX + margin;
  let top = event.clientY + margin;
  if (left + rect.width > window.innerWidth) left = window.innerWidth - rect.width - margin;
  if (top + rect.height > window.innerHeight) top = event.clientY - rect.height - margin;
  tooltip.style.left = `${Math.max(margin, left)}px`;
  tooltip.style.top = `${Math.max(margin, top)}px`;
}

// ---- display-options popup (gear at the text box's top right) --------------

displayBtn.addEventListener("click", (event) => {
  event.stopPropagation();
  displayMenu.hidden = !displayMenu.hidden;
  displayBtn.setAttribute("aria-expanded", String(!displayMenu.hidden));
});

// Clicking checkboxes inside the popup must not dismiss it.
displayMenu.addEventListener("click", (event) => event.stopPropagation());

// Any click elsewhere dismisses the tooltip and the popup.
document.addEventListener("click", () => {
  tooltip.hidden = true;
  displayMenu.hidden = true;
  displayBtn.setAttribute("aria-expanded", "false");
});

// ---- display toggles (flip a class on <body>) ------------------------------

// Adds `className` to <body> when the box is checked (addWhenChecked=true)
// or when it is unchecked (false). "hide-*" classes use the unchecked sense;
// "show-spaces" uses the checked sense.
function wireToggle(checkbox, className, addWhenChecked) {
  const apply = () => {
    const present = addWhenChecked ? checkbox.checked : !checkbox.checked;
    document.body.classList.toggle(className, present);
  };
  checkbox.addEventListener("change", apply);
  apply();
}

wireToggle(document.getElementById("optFurigana"), "hide-furigana", false);
wireToggle(document.getElementById("optGloss"), "hide-gloss", false);
wireToggle(document.getElementById("optSpaces"), "show-spaces", true);
// Unchecked (default) hides particle glosses; checking shows them.
wireToggle(document.getElementById("optShowParticles"), "hide-particles", false);

// ---- kanji histogram mode: a class on <body>, CSS shows one of the two
// pre-rendered histograms. Delegated so the swapped-in buttons keep working.

document.addEventListener("click", (event) => {
  const button = event.target.closest("#kanjiMode button");
  if (!button) return;
  document.body.classList.toggle("wk-mode", button.dataset.mode === "wk");
});
