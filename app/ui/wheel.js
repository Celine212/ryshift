// The wheel: draws slices for the dragged files, highlights the one under the
// cursor (or the dragged file), and runs it on drop or click.

const tauri = window.__TAURI__;
const SVG = "http://www.w3.org/2000/svg";
const C = 160;          // centre of the 320px window
const R_IN = 46;        // hub edge
const R_OUT = 150;      // outer edge
const WHEELS = ["convert", "tools"];
const WHEEL_NAMES = { convert: "CONVERT", tools: "TOOLS" };

const wheelEl = document.getElementById("wheel");
const slicesEl = document.getElementById("slices");
const hubTop = document.getElementById("hub-top");
const hubBottom = document.getElementById("hub-bottom");

let state = { paths: [], actions: [], size: 0, mode: "drag", wheel: "convert", hot: -1, busy: false };

function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024, i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  return `${v >= 10 ? Math.round(v) : v.toFixed(1)} ${units[i]}`;
}

function wheelActions(wheel = state.wheel) {
  return state.actions.filter((a) => a.wheel === wheel).slice(0, 8);
}

function availableWheels() {
  return WHEELS.filter((w) => wheelActions(w).length > 0);
}

function arc(a0, a1, r0, r1) {
  const p = (a, r) => `${C + r * Math.cos(a)} ${C + r * Math.sin(a)}`;
  const large = a1 - a0 > Math.PI ? 1 : 0;
  return `M${p(a0, r1)}A${r1} ${r1} 0 ${large} 1 ${p(a1, r1)}L${p(a1, r0)}A${r0} ${r0} 0 ${large} 0 ${p(a0, r0)}Z`;
}

function setHub(top, bottom) {
  hubTop.textContent = top;
  hubBottom.textContent = bottom;
}

function restingHub() {
  if (!state.paths.length) return setHub("Select", "A FILE");
  if (!state.actions.length) return setHub("No options", "FOR THIS FILE");
  const count = state.paths.length > 1 ? `${state.paths.length} files · ` : "";
  setHub(count + formatSize(state.size), WHEEL_NAMES[state.wheel]);
}

function render() {
  slicesEl.replaceChildren();
  const list = wheelActions();
  const n = list.length;
  const gap = n > 1 ? 0.035 : 0;
  list.forEach((action, i) => {
    const a0 = -Math.PI / 2 - Math.PI / n + (i * 2 * Math.PI) / n + gap / 2;
    const a1 = a0 + (2 * Math.PI) / n - gap;
    const mid = (a0 + a1) / 2;
    const rText = (R_IN + R_OUT) / 2 + 4;
    const g = document.createElementNS(SVG, "g");
    g.setAttribute("class", "slice" + (i === state.hot ? " hot" : ""));
    g.dataset.index = i;
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", n === 1 ? arc(0, Math.PI * 1.999, R_IN, R_OUT) : arc(a0, a1, R_IN, R_OUT));
    const label = document.createElementNS(SVG, "text");
    label.setAttribute("x", C + rText * Math.cos(mid));
    label.setAttribute("y", C + rText * Math.sin(mid) + 4);
    label.setAttribute("text-anchor", "middle");
    label.textContent = action.title.toUpperCase();
    g.append(path, label);
    if (state.mode === "shortcut") {
      const key = document.createElementNS(SVG, "text");
      key.setAttribute("class", "key");
      key.setAttribute("x", C + (R_OUT - 14) * Math.cos(mid));
      key.setAttribute("y", C + (R_OUT - 14) * Math.sin(mid) + 3);
      key.setAttribute("text-anchor", "middle");
      key.textContent = i + 1;
      g.append(key);
    }
    g.addEventListener("mouseenter", () => setHot(i));
    g.addEventListener("mouseleave", () => setHot(-1));
    g.addEventListener("click", () => runSlice(i));
    slicesEl.append(g);
  });
  if (state.hot >= 0 && list[state.hot]) setHub(list[state.hot].title, "DROP TO RUN");
  else restingHub();
}

function setHot(i) {
  if (state.busy || state.hot === i) return;
  state.hot = i;
  render();
}

/// Which slice is under a point in window coordinates (-1 for hub or outside).
function sliceAt(x, y) {
  const dx = x - C, dy = y - C;
  const r = Math.hypot(dx, dy);
  const n = wheelActions().length;
  if (r < R_IN || r > R_OUT || n === 0) return -1;
  let a = Math.atan2(dy, dx) + Math.PI / 2 + Math.PI / n;
  a = ((a % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  return Math.floor(a / ((2 * Math.PI) / n));
}

function hide() {
  wheelEl.classList.remove("open", "busy");
  state.hot = -1;
  state.busy = false;
  tauri.core.invoke("hide_wheel");
}

function shortName(path) {
  const name = path.split(/[\\/]/).pop();
  return name.length > 14 ? name.slice(0, 12) + "…" : name;
}

async function runSlice(i, paths = state.paths) {
  const action = wheelActions()[i];
  if (!action || state.busy || !paths.length) return hide();
  state.busy = true;
  state.hot = i;
  render();
  wheelEl.classList.add("busy");
  setHub(action.title, "WORKING…");
  try {
    const created = await tauri.core.invoke("run_action", { id: action.id, paths });
    wheelEl.classList.remove("busy");
    setHub("✓ Saved", created.length > 1 ? `${created.length} FILES` : shortName(created[0]).toUpperCase());
    setTimeout(hide, 750);
  } catch (err) {
    wheelEl.classList.remove("busy");
    setHub("✕ Failed", String(err).slice(0, 18).toUpperCase());
    console.error(err);
    setTimeout(hide, 1800);
  }
}

function cycleWheel(step = 1) {
  const wheels = availableWheels();
  if (wheels.length < 2 || state.busy) return;
  const at = wheels.indexOf(state.wheel);
  state.wheel = wheels[(at + step + wheels.length) % wheels.length];
  state.hot = -1;
  render();
}

tauri.event.listen("wheel:show", ({ payload }) => {
  state = { ...payload, wheel: "convert", hot: -1, busy: false };
  state.wheel = availableWheels()[0] || "convert";
  render();
  // Restart the open animation. A forced layout instead of requestAnimationFrame,
  // because the window may still be hidden (no frames) when this event arrives.
  wheelEl.classList.remove("open");
  void wheelEl.getBoundingClientRect();
  wheelEl.classList.add("open");
});

tauri.event.listen("wheel:cycle", () => cycleWheel());

// The mouse came up somewhere. If the file was dropped on a slice the drop
// handler is already running; otherwise the user let go elsewhere: close.
tauri.event.listen("wheel:mouseup", () => {
  setTimeout(() => { if (!state.busy) hide(); }, 200);
});

tauri.webview.getCurrentWebview().onDragDropEvent(({ payload }) => {
  const scale = window.devicePixelRatio || 1;
  const pos = payload.position ? { x: payload.position.x / scale, y: payload.position.y / scale } : null;
  if (payload.type === "over" || payload.type === "enter") {
    if (pos) setHot(sliceAt(pos.x, pos.y));
  } else if (payload.type === "leave") {
    setHot(-1);
  } else if (payload.type === "drop") {
    const i = pos ? sliceAt(pos.x, pos.y) : -1;
    if (i < 0) return hide();
    runSlice(i, payload.paths.length ? payload.paths : state.paths);
  }
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") return hide();
  if (e.key === "Tab" || e.key === "Shift") { e.preventDefault(); return cycleWheel(e.shiftKey && e.key === "Tab" ? -1 : 1); }
  const n = Number(e.key);
  if (n >= 1 && n <= 8) runSlice(n - 1);
});

wheelEl.addEventListener("click", (e) => {
  if (e.target.id === "hub" || e.target === wheelEl) hide();
});

window.addEventListener("blur", () => { if (state.mode === "shortcut" && !state.busy) hide(); });
