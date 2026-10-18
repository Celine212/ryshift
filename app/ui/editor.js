// Ryshift editor panels: Crop, Edit, Annotate, Add background, Redact.
//
// The app hands over one image (PDFs give page 1). Every edit happens here on
// a canvas, kept in image coordinates, so the preview (scaled down to stay
// fast) and the saved file (full size) come from the same drawing code.

const tauri = window.__TAURI__;
const PARAMS = window.__RYSHIFT_EDIT__ || {};
const $ = (id) => document.getElementById(id);
const view = $("view");
const ctx = view.getContext("2d");

/** Longest side of the on-screen preview canvas, in canvas pixels. */
const VIEW_MAX = 1600;

/** Rynovi colours for drawing tools. */
const INKS = ["#6f5be0", "#b4446c", "#3f8e2e", "#17330f", "#f4c45a", "#ffffff"];

/** Add-background choices: two stops make a diagonal gradient, one is solid. */
const BACKGROUNDS = [
  ["#7c6cf0", "#4b55d8"],
  ["#f3e4cf", "#d9c19f"],
  ["#ffdbe6", "#ef8fb0"],
  ["#dff3d5", "#7fcf73"],
  ["#d6efff", "#8fc3ef"],
  ["#fff8f1"],
  ["#17330f"],
];

const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

let img = null;            // the decoded source image
let src = null;            // { width, height, ext, name }
let doc = { w: 1, h: 1 };  // size of the image being produced, in output pixels
let k = 1;                 // preview canvas pixels per output pixel
let mode = null;

// ---------- small UI builders ----------

function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  Object.assign(node, props);
  for (const c of [].concat(children)) node.append(c);
  return node;
}

function group(title, ...children) {
  return el("div", { className: "group" }, [title ? el("h2", { textContent: title }) : "", ...children]);
}

/** A row of toggle buttons. `onPick(value)` fires on click. */
function chips(options, current, onPick) {
  const row = el("div", { className: "chips" });
  const buttons = options.map(([label, value]) => {
    const b = el("button", { type: "button", textContent: label });
    b.dataset.value = String(value);
    b.addEventListener("click", () => {
      buttons.forEach((x) => x.classList.toggle("on", x === b));
      onPick(value);
    });
    if (value === current) b.classList.add("on");
    return b;
  });
  row.append(...buttons);
  return row;
}

/** One-shot buttons (Undo, Rotate…). */
function actions(list) {
  return el("div", { className: "chips" }, list.map(([label, fn]) => {
    const b = el("button", { type: "button", textContent: label });
    b.addEventListener("click", fn);
    return b;
  }));
}

function slider(label, min, max, value, format, onInput) {
  const out = el("output", { textContent: format(value) });
  const input = el("input", { type: "range", min, max, step: 1, value });
  input.addEventListener("input", () => {
    out.textContent = format(Number(input.value));
    onInput(Number(input.value));
  });
  return el("label", { className: "slider" }, [el("span", {}, [label, out]), input]);
}

function swatches(colors, current, onPick) {
  const row = el("div", { className: "chips" });
  const buttons = colors.map((c, i) => {
    const b = el("button", { type: "button", className: "swatch", title: c.join ? c.join(" → ") : c });
    b.style.background = Array.isArray(c) && c.length > 1 ? `linear-gradient(135deg, ${c[0]}, ${c[1]})` : [].concat(c)[0];
    b.addEventListener("click", () => {
      buttons.forEach((x) => x.classList.toggle("on", x === b));
      onPick(i);
    });
    if (i === current) b.classList.add("on");
    return b;
  });
  row.append(...buttons);
  return row;
}

function setStatus(text, error = false) {
  $("status").textContent = text;
  $("status").classList.toggle("error", error);
}

// ---------- canvas plumbing ----------

/** Size the output image; the preview canvas follows, scaled to stay fast. */
function setDoc(w, h) {
  doc = { w: Math.max(1, Math.round(w)), h: Math.max(1, Math.round(h)) };
  k = Math.min(1, VIEW_MAX / Math.max(doc.w, doc.h));
  view.width = Math.max(1, Math.round(doc.w * k));
  view.height = Math.max(1, Math.round(doc.h * k));
  fit();
}

/** Show the preview canvas as large as the stage allows, keeping its shape. */
function fit() {
  const stage = $("stage").getBoundingClientRect();
  const room = { w: Math.max(50, stage.width - 32), h: Math.max(50, stage.height - 32) };
  const s = Math.min(room.w / view.width, room.h / view.height);
  view.style.width = `${Math.round(view.width * s)}px`;
  view.style.height = `${Math.round(view.height * s)}px`;
}

/** Pointer position in output pixels. */
function toDoc(e) {
  const r = view.getBoundingClientRect();
  return { x: ((e.clientX - r.left) / r.width) * doc.w, y: ((e.clientY - r.top) / r.height) * doc.h };
}

/** How many output pixels one screen pixel covers (for handle sizes). */
function screenUnit() {
  const r = view.getBoundingClientRect();
  return r.width ? doc.w / r.width : 1;
}

function render() {
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, view.width, view.height);
  mode.paint(ctx, k, true);
  setStatus(mode.status());
}

/** A size that looks the same on small and huge images (≈1px on an 800px image). */
function unit() {
  return Math.max(src.width, src.height) / 800;
}

// ---------- Crop ----------

const Crop = {
  title: "Crop image",
  suffix: "Cropped",
  rect: null,
  ratio: null,
  drag: null,

  start() {
    setDoc(src.width, src.height);
    this.rect = { x: 0, y: 0, w: src.width, h: src.height };
    view.classList.add("cross");
    this.sizeLabel = el("span", { className: "dims" });
    return [
      group("Ratio", chips([["Free", null], ["1:1", 1], ["4:5", 4 / 5], ["9:16", 9 / 16], ["16:9", 16 / 9], ["Original", "original"]], null,
        (v) => this.setRatio(v))),
      group("Size", this.sizeLabel),
      el("p", { className: "hint", textContent: "Drag the box or its corners. Drag outside it to start a new box. Enter saves." }),
    ];
  },

  setRatio(v) {
    this.ratio = v === "original" ? src.width / src.height : v;
    if (!this.ratio) return render();
    // Largest box of that shape, centred on the current box, inside the image.
    const cx = this.rect.x + this.rect.w / 2, cy = this.rect.y + this.rect.h / 2;
    let w = src.width, h = w / this.ratio;
    if (h > src.height) { h = src.height; w = h * this.ratio; }
    this.rect = this.inBounds({ x: cx - w / 2, y: cy - h / 2, w, h });
    render();
  },

  /** Shift (and if needed shrink) a box so it sits inside the image. */
  inBounds(r) {
    let { x, y, w, h } = r;
    if (w > src.width) { w = src.width; if (this.ratio) h = w / this.ratio; }
    if (h > src.height) { h = src.height; if (this.ratio) w = h * this.ratio; }
    return { x: clamp(x, 0, src.width - w), y: clamp(y, 0, src.height - h), w, h };
  },

  handles() {
    const { x, y, w, h } = this.rect;
    return {
      nw: [x, y], n: [x + w / 2, y], ne: [x + w, y], e: [x + w, y + h / 2],
      se: [x + w, y + h], s: [x + w / 2, y + h], sw: [x, y + h], w: [x, y + h / 2],
    };
  },

  down(p) {
    const reach = 12 * screenUnit();
    for (const [name, [hx, hy]] of Object.entries(this.handles())) {
      if (Math.abs(p.x - hx) <= reach && Math.abs(p.y - hy) <= reach) {
        this.drag = { kind: "handle", name, p, start: { ...this.rect } };
        return;
      }
    }
    const r = this.rect;
    const inside = p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h;
    this.drag = inside ? { kind: "move", p, start: { ...r } } : { kind: "new", p };
  },

  move(p) {
    const d = this.drag;
    if (!d) return;
    const min = 8;
    if (d.kind === "move") {
      this.rect = this.inBounds({ ...d.start, x: d.start.x + p.x - d.p.x, y: d.start.y + p.y - d.p.y });
    } else if (d.kind === "new") {
      let w = Math.abs(p.x - d.p.x), h = Math.abs(p.y - d.p.y);
      if (this.ratio) h = w / this.ratio;
      const x = p.x < d.p.x ? d.p.x - w : d.p.x, y = p.y < d.p.y ? d.p.y - h : d.p.y;
      if (w >= min && h >= min) this.rect = this.inBounds(this.clipNew({ x, y, w, h }));
    } else {
      this.rect = this.resized(d, p);
    }
  },

  /** A freshly drawn box can't start outside the image. */
  clipNew(r) {
    const x0 = clamp(r.x, 0, src.width), y0 = clamp(r.y, 0, src.height);
    let w = clamp(r.x + r.w, 0, src.width) - x0, h = clamp(r.y + r.h, 0, src.height) - y0;
    if (this.ratio) { if (w / h > this.ratio) w = h * this.ratio; else h = w / this.ratio; }
    return { x: x0, y: y0, w: Math.max(8, w), h: Math.max(8, h) };
  },

  resized(d, p) {
    const s = d.start, name = d.name, min = 8;
    let l = s.x, t = s.y, r = s.x + s.w, b = s.y + s.h;
    const dx = p.x - d.p.x, dy = p.y - d.p.y;
    if (name.includes("w")) l = clamp(l + dx, 0, r - min);
    if (name.includes("e")) r = clamp(r + dx, l + min, src.width);
    if (name.includes("n")) t = clamp(t + dy, 0, b - min);
    if (name.includes("s")) b = clamp(b + dy, t + min, src.height);
    if (!this.ratio) return { x: l, y: t, w: r - l, h: b - t };

    // Keep the shape: the dragged axis decides, the opposite side stays put.
    let w = r - l, h = b - t;
    if (name === "n" || name === "s") w = h * this.ratio; else h = w / this.ratio;
    // Room available from the fixed side (or centre) in each direction.
    const cx = s.x + s.w / 2, cy = s.y + s.h / 2;
    const roomW = name.includes("w") ? r : name.includes("e") ? src.width - l : 2 * Math.min(cx, src.width - cx);
    const roomH = name.includes("n") ? b : name.includes("s") ? src.height - t : 2 * Math.min(cy, src.height - cy);
    const scale = Math.min(1, roomW / w, roomH / h);
    w *= scale; h *= scale;
    const x = name.includes("w") ? r - w : name.includes("e") ? l : cx - w / 2;
    const y = name.includes("n") ? b - h : name.includes("s") ? t : cy - h / 2;
    return { x, y, w: Math.max(min, w), h: Math.max(min, h) };
  },

  up() { this.drag = null; },

  outputSize() { return { w: Math.round(this.rect.w), h: Math.round(this.rect.h) }; },

  paint(c, s, preview) {
    const r = this.rect;
    c.setTransform(s, 0, 0, s, 0, 0);
    if (!preview) {
      c.drawImage(img, r.x, r.y, r.w, r.h, 0, 0, r.w, r.h);
      return;
    }
    c.drawImage(img, 0, 0);
    const u = screenUnit();
    c.fillStyle = "rgba(23, 51, 15, 0.5)";
    c.beginPath();
    c.rect(0, 0, src.width, src.height);
    c.rect(r.x, r.y, r.w, r.h);
    c.fill("evenodd");
    c.strokeStyle = "rgba(255, 248, 241, 0.55)";
    c.lineWidth = u;
    c.beginPath();
    for (const f of [1 / 3, 2 / 3]) {
      c.moveTo(r.x + r.w * f, r.y); c.lineTo(r.x + r.w * f, r.y + r.h);
      c.moveTo(r.x, r.y + r.h * f); c.lineTo(r.x + r.w, r.y + r.h * f);
    }
    c.stroke();
    c.strokeStyle = "#fff8f1";
    c.lineWidth = 2 * u;
    c.strokeRect(r.x, r.y, r.w, r.h);
    const hs = 12 * u;
    for (const [hx, hy] of Object.values(this.handles())) {
      c.fillStyle = "#fff8f1";
      c.strokeStyle = "#6f5be0";
      c.lineWidth = 2 * u;
      c.beginPath();
      c.roundRect(hx - hs / 2, hy - hs / 2, hs, hs, 3 * u);
      c.fill();
      c.stroke();
    }
    this.sizeLabel.textContent = `${Math.round(r.w)} × ${Math.round(r.h)} px`;
  },

  status() { return "Saves next to the original"; },
};

// ---------- Edit ----------

const Edit = {
  title: "Edit image",
  suffix: "Edited",
  rot: 0,
  flipH: false,
  flipV: false,
  bright: 0,
  contrast: 0,
  sat: 0,
  size: 1,

  start() {
    this.resize();
    const signed = (v) => (v > 0 ? `+${v}` : `${v}`);
    return [
      group("Rotate & flip", actions([
        ["⟲ Left", () => { this.rot = (this.rot + 270) % 360; this.resize(); }],
        ["⟳ Right", () => { this.rot = (this.rot + 90) % 360; this.resize(); }],
        ["⇋ Flip", () => { this.flipH = !this.flipH; render(); }],
        ["⇵ Flip", () => { this.flipV = !this.flipV; render(); }],
      ])),
      group("Adjust",
        slider("Brightness", -100, 100, 0, signed, (v) => { this.bright = v; render(); }),
        slider("Contrast", -100, 100, 0, signed, (v) => { this.contrast = v; render(); }),
        slider("Saturation", -100, 100, 0, signed, (v) => { this.sat = v; render(); })),
      group("Size", chips([["100%", 1], ["75%", 0.75], ["50%", 0.5], ["25%", 0.25]], 1, (v) => { this.size = v; this.resize(); })),
    ];
  },

  resize() {
    const turned = this.rot % 180 !== 0;
    const w = (turned ? src.height : src.width) * this.size;
    const h = (turned ? src.width : src.height) * this.size;
    setDoc(w, h);
    render();
  },

  outputSize() { return { w: doc.w, h: doc.h }; },

  paint(c, s) {
    const W = Math.round(doc.w * s), H = Math.round(doc.h * s);
    c.setTransform(1, 0, 0, 1, 0, 0);
    c.translate(W / 2, H / 2);
    c.rotate((this.rot * Math.PI) / 180);
    c.scale(this.flipH ? -1 : 1, this.flipV ? -1 : 1);
    const turned = this.rot % 180 !== 0;
    const dw = turned ? H : W, dh = turned ? W : H;
    c.imageSmoothingQuality = "high";
    c.drawImage(img, -dw / 2, -dh / 2, dw, dh);
    c.setTransform(1, 0, 0, 1, 0, 0);
    if (this.bright || this.contrast || this.sat) {
      const data = c.getImageData(0, 0, W, H);
      adjustPixels(data.data, this.bright, this.contrast, this.sat);
      c.putImageData(data, 0, 0);
    }
  },

  status() { return `${doc.w} × ${doc.h} px`; },
};

/** Brightness, contrast and saturation (each −100…100), in place. Done by hand
 *  so it works the same in every web view (canvas filters aren't everywhere). */
function adjustPixels(px, bright, contrast, sat) {
  const b = bright * 2.55;
  const cv = contrast * 2.55;
  const f = (259 * (cv + 255)) / (255 * (259 - cv));
  const s = 1 + sat / 100;
  for (let i = 0; i < px.length; i += 4) {
    let r = f * (px[i] + b - 128) + 128;
    let g = f * (px[i + 1] + b - 128) + 128;
    let bl = f * (px[i + 2] + b - 128) + 128;
    const gray = 0.299 * r + 0.587 * g + 0.114 * bl;
    px[i] = gray + (r - gray) * s;
    px[i + 1] = gray + (g - gray) * s;
    px[i + 2] = gray + (bl - gray) * s;
  }
}

// ---------- Annotate ----------

const Annotate = {
  title: "Annotate",
  suffix: "Annotated",
  tool: "arrow",
  color: INKS[1],
  thickness: 4,
  text: "Look here",
  marks: [],
  live: null,

  start() {
    setDoc(src.width, src.height);
    view.classList.add("cross");
    const textInput = el("input", { type: "text", value: this.text, placeholder: "Your text" });
    textInput.addEventListener("input", () => { this.text = textInput.value; });
    return [
      group("Tool", chips([["Pen", "pen"], ["Arrow", "arrow"], ["Box", "box"], ["Highlight", "highlight"], ["Text", "text"]], this.tool,
        (v) => { this.tool = v; })),
      group("Colour", swatches(INKS, 1, (i) => { this.color = INKS[i]; })),
      group("Thickness", slider("", 1, 12, this.thickness, (v) => `${v} px`, (v) => { this.thickness = v; })),
      group("Text", textInput, el("p", { className: "hint", textContent: "With the Text tool, click where it should go." })),
      group("", actions([["Undo", () => { this.marks.pop(); render(); }], ["Clear", () => { this.marks = []; render(); }]])),
    ];
  },

  width() { return this.thickness * unit(); },

  down(p) {
    const base = { color: this.color, width: this.width() };
    if (this.tool === "text") {
      if (this.text.trim()) this.marks.push({ ...base, type: "text", x: p.x, y: p.y, text: this.text.trim() });
      return;
    }
    this.live = { ...base, type: this.tool, points: [p, p] };
  },

  move(p) {
    if (!this.live) return;
    if (this.live.type === "pen") this.live.points.push(p);
    else this.live.points[1] = p;
  },

  up() {
    const m = this.live;
    this.live = null;
    if (!m) return;
    const [a, b] = [m.points[0], m.points[m.points.length - 1]];
    if (m.type !== "pen" && Math.hypot(b.x - a.x, b.y - a.y) < 3 * unit()) return; // a click, not a drag
    this.marks.push(m);
  },

  outputSize() { return { w: src.width, h: src.height }; },

  paint(c, s) {
    c.setTransform(s, 0, 0, s, 0, 0);
    c.drawImage(img, 0, 0);
    for (const m of this.live ? [...this.marks, this.live] : this.marks) drawMark(c, m);
  },

  status() { return `${this.marks.length} mark${this.marks.length === 1 ? "" : "s"}`; },
};

function drawMark(c, m) {
  c.save();
  c.strokeStyle = m.color;
  c.fillStyle = m.color;
  c.lineWidth = m.width;
  c.lineCap = "round";
  c.lineJoin = "round";
  const [a, b] = [m.points?.[0], m.points?.[m.points.length - 1]];
  if (m.type === "pen") {
    c.beginPath();
    m.points.forEach((p, i) => (i ? c.lineTo(p.x, p.y) : c.moveTo(p.x, p.y)));
    c.stroke();
  } else if (m.type === "highlight") {
    c.globalAlpha = 0.38;
    c.lineWidth = m.width * 4;
    c.lineCap = "butt";
    c.beginPath(); c.moveTo(a.x, a.y); c.lineTo(b.x, b.y); c.stroke();
  } else if (m.type === "box") {
    c.strokeRect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  } else if (m.type === "arrow") {
    const angle = Math.atan2(b.y - a.y, b.x - a.x);
    const head = Math.max(m.width * 4, 10);
    c.beginPath(); c.moveTo(a.x, a.y); c.lineTo(b.x, b.y);
    for (const side of [-1, 1]) {
      c.moveTo(b.x, b.y);
      c.lineTo(b.x - head * Math.cos(angle + side * 0.45), b.y - head * Math.sin(angle + side * 0.45));
    }
    c.stroke();
  } else if (m.type === "text") {
    c.font = `800 ${Math.max(12, m.width * 6)}px "Inter Tight", system-ui, sans-serif`;
    c.textBaseline = "middle";
    c.fillText(m.text, m.x, m.y);
  }
  c.restore();
}

// ---------- Add background ----------

const AddBg = {
  title: "Add background",
  suffix: "With Background",
  ext: "png", // keeps the rounded corners crisp
  bg: 0,
  shape: 1,
  padding: 12,
  corners: 24,
  shadow: 60,

  start() {
    this.relayout();
    return [
      group("Background", swatches(BACKGROUNDS, this.bg, (i) => { this.bg = i; render(); })),
      group("Shape", chips([["Auto", 0], ["1:1", 1], ["4:5", 4 / 5], ["16:9", 16 / 9], ["9:16", 9 / 16]], this.shape,
        (v) => { this.shape = v; this.relayout(); })),
      group("Style",
        slider("Padding", 0, 30, this.padding, (v) => `${v}%`, (v) => { this.padding = v; this.relayout(); }),
        slider("Corners", 0, 60, this.corners, (v) => `${v} px`, (v) => { this.corners = v; render(); }),
        slider("Shadow", 0, 100, this.shadow, (v) => `${v}`, (v) => { this.shadow = v; render(); })),
    ];
  },

  relayout() {
    const pad = (this.padding / 100) * Math.max(src.width, src.height);
    let w = src.width + 2 * pad, h = src.height + 2 * pad;
    if (this.shape) {
      if (w / h < this.shape) w = h * this.shape; else h = w / this.shape;
    }
    setDoc(w, h);
    render();
  },

  outputSize() { return { w: doc.w, h: doc.h }; },

  paint(c, s) {
    c.setTransform(s, 0, 0, s, 0, 0);
    const stops = BACKGROUNDS[this.bg];
    if (stops.length > 1) {
      const g = c.createLinearGradient(0, 0, doc.w, doc.h);
      g.addColorStop(0, stops[0]);
      g.addColorStop(1, stops[1]);
      c.fillStyle = g;
    } else {
      c.fillStyle = stops[0];
    }
    c.fillRect(0, 0, doc.w, doc.h);

    const u = unit();
    const x = (doc.w - src.width) / 2, y = (doc.h - src.height) / 2;
    const radius = Math.min(this.corners * u, src.width / 2, src.height / 2);
    // Shadow from a solid rounded card under the image.
    if (this.shadow) {
      c.save();
      c.shadowColor = `rgba(20, 10, 60, ${(0.45 * this.shadow) / 100})`;
      c.shadowBlur = this.shadow * u * 0.6 * s;
      c.shadowOffsetY = this.shadow * u * 0.25 * s;
      c.fillStyle = "#fff";
      c.beginPath();
      c.roundRect(x, y, src.width, src.height, radius);
      c.fill();
      c.restore();
    }
    c.save();
    c.beginPath();
    c.roundRect(x, y, src.width, src.height, radius);
    c.clip();
    c.drawImage(img, x, y);
    c.restore();
  },

  status() { return `${doc.w} × ${doc.h} px`; },
};

// ---------- Redact ----------

const Redact = {
  title: "Redact",
  suffix: "Redacted",
  style: "black",
  boxes: [],
  live: null,

  start() {
    setDoc(src.width, src.height);
    view.classList.add("cross");
    return [
      group("Style", chips([["Black box", "black"], ["Pixelate", "pixelate"]], this.style, (v) => { this.style = v; })),
      el("p", { className: "hint", textContent: "Drag over anything private. Black is the safest for passwords and account numbers." }),
      group("", actions([["Undo", () => { this.boxes.pop(); render(); }], ["Clear", () => { this.boxes = []; render(); }]])),
    ];
  },

  down(p) { this.live = { a: p, b: p, style: this.style }; },
  move(p) { if (this.live) this.live.b = p; },
  up() {
    const l = this.live;
    this.live = null;
    if (!l) return;
    const box = normalize(l.a, l.b);
    if (box.w >= 3 && box.h >= 3) this.boxes.push({ ...box, style: l.style });
  },

  outputSize() { return { w: src.width, h: src.height }; },

  paint(c, s, preview) {
    c.setTransform(s, 0, 0, s, 0, 0);
    c.drawImage(img, 0, 0);
    const all = this.live ? [...this.boxes, { ...normalize(this.live.a, this.live.b), style: this.live.style, live: true }] : this.boxes;
    for (const b of all) {
      if (b.w < 1 || b.h < 1) continue;
      if (b.style === "pixelate") pixelate(c, b);
      else { c.fillStyle = "#000"; c.fillRect(b.x, b.y, b.w, b.h); }
      if (preview && b.live) {
        c.strokeStyle = "#6f5be0";
        c.lineWidth = 2 * screenUnit();
        c.strokeRect(b.x, b.y, b.w, b.h);
      }
    }
  },

  status() { return `${this.boxes.length} area${this.boxes.length === 1 ? "" : "s"} hidden`; },
};

/** Blocky copy of one area of the source image. */
function pixelate(c, b) {
  const block = Math.max(6, Math.round(Math.min(b.w, b.h) / 5));
  const tw = Math.max(1, Math.round(b.w / block)), th = Math.max(1, Math.round(b.h / block));
  const tiny = document.createElement("canvas");
  tiny.width = tw;
  tiny.height = th;
  tiny.getContext("2d").drawImage(img, b.x, b.y, b.w, b.h, 0, 0, tw, th);
  c.save();
  c.imageSmoothingEnabled = false;
  c.drawImage(tiny, 0, 0, tw, th, b.x, b.y, b.w, b.h);
  c.restore();
}

function normalize(a, b) {
  const x = clamp(Math.min(a.x, b.x), 0, src.width), y = clamp(Math.min(a.y, b.y), 0, src.height);
  return { x, y, w: clamp(Math.max(a.x, b.x), 0, src.width) - x, h: clamp(Math.max(a.y, b.y), 0, src.height) - y };
}

// ---------- saving ----------

const MODES = { crop: Crop, edit: Edit, annotate: Annotate, addbg: AddBg, redact: Redact };

/** Draw the full-size result and return it as base64 bytes. */
async function exportImage() {
  const { w, h } = mode.outputSize();
  const out = document.createElement("canvas");
  out.width = w;
  out.height = h;
  const oc = out.getContext("2d");
  const ext = mode.ext || src.ext;
  if (ext === "jpg") { oc.fillStyle = "#fff"; oc.fillRect(0, 0, w, h); }
  mode.paint(oc, 1, false);
  const blob = await new Promise((resolve) => out.toBlob(resolve, ext === "jpg" ? "image/jpeg" : "image/png", 0.92));
  const dataUrl = await new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result);
    reader.readAsDataURL(blob);
  });
  return { ext, data: String(dataUrl).split(",")[1] };
}

async function save() {
  const button = $("save");
  if (button.disabled) return;
  button.disabled = true;
  setStatus("Saving…");
  try {
    const { ext, data } = await exportImage();
    const saved = await tauri.core.invoke("save_edit", { path: PARAMS.path, suffix: mode.suffix, ext, data });
    setStatus(`Saved ${saved.split(/[\\/]/).pop()}`);
    setTimeout(close, 700);
  } catch (err) {
    setStatus(`Couldn't save: ${err}`, true);
    button.disabled = false;
  }
}

function close() {
  tauri.window.getCurrentWindow().close();
}

// ---------- start ----------

async function init() {
  mode = MODES[PARAMS.mode];
  if (!mode) return setStatus(`Unknown editor: ${PARAMS.mode}`, true);
  $("title").textContent = mode.title;
  document.title = mode.title;
  $("save").textContent = PARAMS.mode === "crop" ? "Crop" : "Save";
  try {
    src = await tauri.core.invoke("load_for_edit", { path: PARAMS.path });
  } catch (err) {
    $("loading").textContent = "";
    return setStatus(`Couldn't open this file: ${err}`, true);
  }
  $("filename").textContent = src.name + (/\.pdf$/i.test(src.name) ? " · page 1" : "");
  img = new Image();
  img.onload = () => {
    $("loading").remove();
    $("controls").append(...mode.start());
    $("save").disabled = false;
    render();
  };
  img.onerror = () => setStatus("Couldn't read this image", true);
  img.src = src.data_url;
}

// Track the drag ourselves; pointer capture is a bonus where the web view allows it.
let dragging = false;
view.addEventListener("pointerdown", (e) => {
  if (!mode?.down || !img || e.button > 0) return;
  dragging = true;
  try { view.setPointerCapture(e.pointerId); } catch { /* synthetic or unsupported */ }
  mode.down(toDoc(e));
  render();
});
window.addEventListener("pointermove", (e) => {
  if (!dragging || !mode?.move) return;
  mode.move(toDoc(e));
  render();
});
window.addEventListener("pointerup", (e) => {
  if (!dragging) return;
  dragging = false;
  mode.up?.(toDoc(e));
  render();
});

$("save").addEventListener("click", save);
$("cancel").addEventListener("click", close);
document.addEventListener("keydown", (e) => {
  if (e.target.tagName === "INPUT" && e.target.type === "text") return;
  if (e.key === "Escape") close();
  if (e.key === "Enter") save();
  if ((e.metaKey || e.ctrlKey) && e.key === "z" && mode?.marks) { mode.marks.pop(); render(); }
});
window.addEventListener("resize", () => { if (img) fit(); });

init();
