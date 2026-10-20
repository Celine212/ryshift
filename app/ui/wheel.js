const tauri = window.__TAURI__;
const SVG = "http://www.w3.org/2000/svg";
const C = 160;
const R_IN = 46;
const R_OUT = 150;
const WHEELS = ["convert", "tools"];
const WHEEL_NAMES = { convert: "CONVERT", tools: "TOOLS" };

const rootEl = document.getElementById("root");
const wheelEl = document.getElementById("wheel");
const slicesEl = document.getElementById("slices");
const hubTop = document.getElementById("hub-top");
const hubBottom = document.getElementById("hub-bottom");

let state = { paths: [], actions: [], size: 0, mode: "drag", wheel: "convert", hot: -1, busy: false, fired: false };

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

const P_IN = 54;
const P_OUT = 142;
const P_ROUND = 8;

const ICONS = {
  image: '<rect x="3" y="4" width="18" height="16" rx="3"/><circle cx="9" cy="10" r="1.8"/><path d="M4 18l6-6 4 4 3-3 4 4"/>',
  doc: '<path d="M6 3h8l5 5v13H6z"/><path d="M14 3v5h5"/>',
  txt: '<path d="M6 3h8l5 5v13H6z"/><path d="M14 3v5h5"/><path d="M9 13h7M9 17h7"/>',
  crop: '<path d="M6 2v14a2 2 0 0 0 2 2h14"/><path d="M18 22V8a2 2 0 0 0-2-2H2"/>',
  compress: '<path d="M4 14h6v6"/><path d="M20 10h-6V4"/><path d="M10 14l-6 6"/><path d="M14 10l6-6"/>',
  tag: '<path d="M3 12V4h8l10 10-8 8z"/><circle cx="7.5" cy="7.5" r="1.5"/>',
  sliders: '<path d="M4 6h16M4 12h16M4 18h16"/><circle cx="9" cy="6" r="2.2"/><circle cx="15" cy="12" r="2.2"/><circle cx="7" cy="18" r="2.2"/>',
  pen: '<path d="M4 20l4-1 11-11-3-3L5 16z"/><path d="M14 6l3 3"/>',
  frame: '<rect x="2.5" y="2.5" width="19" height="19" rx="4"/><rect x="7" y="7" width="10" height="10" rx="2"/>',
  redact: '<path d="M3 12s3.5-6 9-6 9 6 9 6-3.5 6-9 6-9-6-9-6z"/><circle cx="12" cy="12" r="2.5"/><path d="M4 20L20 4"/>',
  tool: '<circle cx="12" cy="12" r="8"/><path d="M12 8v8M8 12h8"/>',
};

function iconFor(id) {
  const map = {
    "convert.pdf": "doc", "convert.txt": "txt", "tool.crop": "crop", "tool.compress": "compress",
    "tool.strip": "tag", "tool.edit": "sliders", "tool.annotate": "pen", "tool.addbg": "frame", "tool.redact": "redact",
  };
  return map[id] || (id.startsWith("convert.") ? "image" : "tool");
}

function petalPath(a0, a1, n) {
  const p = (a, r) => `${(C + r * Math.cos(a)).toFixed(2)} ${(C + r * Math.sin(a)).toFixed(2)}`;
  const ri = P_IN + P_ROUND, ro = P_OUT - P_ROUND;
  if (n === 1) {
    return `M${C + ro} ${C}A${ro} ${ro} 0 1 1 ${C - ro} ${C}A${ro} ${ro} 0 1 1 ${C + ro} ${C}Z` +
      `M${C + ri} ${C}A${ri} ${ri} 0 1 0 ${C - ri} ${C}A${ri} ${ri} 0 1 0 ${C + ri} ${C}Z`;
  }
  const pad = (r) => (P_ROUND + 3) / r;
  const b0o = a0 + pad(ro), b1o = a1 - pad(ro), b0i = a0 + pad(ri), b1i = a1 - pad(ri);
  const large = b1o - b0o > Math.PI ? 1 : 0;
  return `M${p(b0o, ro)}A${ro} ${ro} 0 ${large} 1 ${p(b1o, ro)}L${p(b1i, ri)}A${ri} ${ri} 0 ${large} 0 ${p(b0i, ri)}Z`;
}

function setHub(top, bottom) {
  hubTop.textContent = top;
  hubBottom.textContent = bottom;
}

function restingHub() {
  if (!state.paths.length && state.mode === "drag") return setHub("Ryshift", "READING FILE");
  if (!state.paths.length) return setHub("Select", "A FILE");
  if (!state.actions.length) return setHub("No options", "FOR THIS FILE");
  const count = state.paths.length > 1 ? `${state.paths.length} files · ` : "";
  setHub(count + formatSize(state.size), WHEEL_NAMES[state.wheel]);
}

function render() {
  slicesEl.replaceChildren();
  const list = wheelActions();
  const n = list.length;
  list.forEach((action, i) => {
    const a0 = -Math.PI / 2 - Math.PI / n + (i * 2 * Math.PI) / n;
    const a1 = a0 + (2 * Math.PI) / n;
    const mid = (a0 + a1) / 2;
    const rMid = 98;
    const cx = n === 1 ? C : C + rMid * Math.cos(mid);
    const cy = n === 1 ? C - rMid : C + rMid * Math.sin(mid);
    const g = document.createElementNS(SVG, "g");
    g.setAttribute("class", "slice" + (i === state.hot ? " hot" : ""));
    g.dataset.index = i;
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("class", "petal");
    path.setAttribute("d", petalPath(a0, a1, n));
    path.setAttribute("fill-rule", "evenodd");
    const icon = document.createElementNS(SVG, "g");
    icon.setAttribute("class", "icon");
    icon.setAttribute("transform", `translate(${cx - 10} ${cy - 20}) scale(0.84)`);
    icon.innerHTML = ICONS[iconFor(action.id)];
    const label = document.createElementNS(SVG, "text");
    label.setAttribute("x", cx);
    label.setAttribute("y", cy + 15);
    label.setAttribute("text-anchor", "middle");
    label.textContent = action.title.toUpperCase();
    g.append(path, icon, label);
    if (state.mode === "shortcut") {
      const key = document.createElementNS(SVG, "text");
      key.setAttribute("class", "key");
      key.setAttribute("x", cx);
      key.setAttribute("y", cy + 28);
      key.setAttribute("text-anchor", "middle");
      key.textContent = i + 1;
      g.append(key);
    }
    g.addEventListener("mouseenter", () => setHot(i));
    g.addEventListener("mouseleave", () => setHot(-1));
    g.addEventListener("click", () => runSlice(i));
    slicesEl.append(g);
  });
  if (state.hot >= 0 && list[state.hot]) setHub(list[state.hot].title, list[state.hot].panel ? "DROP TO OPEN" : "DROP TO RUN");
  else restingHub();
}

function setHot(i) {
  if (state.busy || state.hot === i) return;
  state.hot = i;
  render();
}

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
  rootEl.classList.remove("open", "busy");
  state.hot = -1;
  state.busy = false;
  tauri.core.invoke("hide_wheel");
}

function shortName(path) {
  const name = path.split(/[\\/]/).pop();
  return name.length > 14 ? name.slice(0, 12) + "…" : name;
}

async function runSlice(i, paths = state.paths) {
  if (state.fired) return; // the drop and the mouse release can both arrive; run once
  const action = wheelActions()[i];
  if (!action || state.busy || !paths.length) return hide();
  state.fired = true;
  if (action.panel) {
    try {
      await tauri.core.invoke("open_panel", { panel: action.panel, path: paths[0] });
    } catch (err) {
      console.error(err);
    }
    return hide();
  }
  state.busy = true;
  state.hot = i;
  render();
  rootEl.classList.add("busy");
  setHub(action.title, "WORKING…");
  try {
    const created = await tauri.core.invoke("run_action", { id: action.id, paths });
    rootEl.classList.remove("busy");
    setHub("✓ Saved", created.length > 1 ? `${created.length} FILES` : shortName(created[0]).toUpperCase());
    setTimeout(hide, 750);
  } catch (err) {
    rootEl.classList.remove("busy");
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
  state = { ...payload, wheel: "convert", hot: -1, busy: false, fired: false };
  state.wheel = availableWheels()[0] || "convert";
  document.body.classList.toggle("native-glass", !!payload.native_glass);
  render();
  rootEl.classList.remove("open");
  void rootEl.getBoundingClientRect();
  rootEl.classList.add("open");
});

tauri.event.listen("wheel:cycle", () => cycleWheel());

function toCss(p) {
  const scale = state.drop_in_pixels ? window.devicePixelRatio || 1 : 1;
  return p ? { x: p.x / scale, y: p.y / scale } : null;
}

tauri.event.listen("wheel:pointer", ({ payload }) => {
  const pos = toCss(payload);
  if (pos) setHot(sliceAt(pos.x, pos.y));
});

tauri.event.listen("wheel:mouseup", ({ payload }) => {
  const pos = toCss(payload);
  const i = pos ? sliceAt(pos.x, pos.y) : -1;
  setTimeout(() => {
    if (state.fired || state.busy) return;
    if (i >= 0 && state.paths.length) runSlice(i);
    else hide();
  }, 200);
});

tauri.webview.getCurrentWebview().onDragDropEvent(({ payload }) => {
  const scale = state.drop_in_pixels ? window.devicePixelRatio || 1 : 1;
  const pos = payload.position ? { x: payload.position.x / scale, y: payload.position.y / scale } : null;
  if (payload.type === "enter" && payload.paths?.length && !state.paths.length) {
    state.paths = payload.paths;
    tauri.core.invoke("actions_for", { paths: payload.paths }).then(({ actions, size }) => {
      state.actions = actions;
      state.size = size;
      state.wheel = availableWheels()[0] || "convert";
      render();
    });
  }
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
  if (e.target.id === "hub" || e.target === wheelEl || e.target.id === "sheen-disk") hide();
});

window.addEventListener("blur", () => { if (state.mode === "shortcut" && !state.busy) hide(); });
