const $ = (id) => document.getElementById(id);
const input = $("input"), status = $("status"), canvas = $("c");
const ctx = canvas.getContext("2d");

const EXAMPLES = {
  ex1: "a b\na c\na d\na e\na f\nc f\nc j\nd e\nd g\nf i\nh i\nb h\n",
  ex2: "0 1\n1 2\n2 3\n3 4\n4 0\n0 5\n1 6\n2 7\n3 8\n4 9\n5 7\n7 9\n9 6\n6 8\n8 5\n",
};

// Same format as the command line tool.
function parse(text) {
  const names = [], index = new Map(), edges = [];
  for (const raw of text.split("\n")) {
    const ids = raw.split("#")[0].split(/[\s,]+/).filter(Boolean).map((name) => {
      if (!index.has(name)) { index.set(name, names.length); names.push(name); }
      return index.get(name);
    });
    for (let i = 0; i + 1 < ids.length; i++) edges.push([ids[i], ids[i + 1]]);
  }
  return { names, edges };
}

// --- Layout: force-directed with cooling ------------------------------------
let graph = { names: [], edges: [] }, adj = [], pos = [], solution = new Set();
let heat = 0, dirty = true;

function reheat(amount = 1) { heat = Math.max(heat, amount); dirty = true; }

function setGraph(g, keepSolution = false) {
  const old = new Map(graph.names.map((name, i) => [name, pos[i]]));
  graph = g;
  adj = g.names.map(() => []);
  for (const [i, j] of g.edges) { adj[i].push(j); adj[j].push(i); }
  const n = g.names.length;
  pos = g.names.map((name, i) => old.get(name) ?? {
    // Start on a spiral so the layout unfolds evenly instead of from noise.
    x: Math.cos(i * 2.39996) * Math.sqrt((i + 0.5) / Math.max(n, 1)) * 0.5,
    y: Math.sin(i * 2.39996) * Math.sqrt((i + 0.5) / Math.max(n, 1)) * 0.5,
    vx: 0, vy: 0,
  });
  if (!keepSolution) { solution = new Set(); $("result").hidden = true; }
  hover = -1;
  $("counts").textContent = n ? `${n} V · ${g.edges.length} E` : "";
  reheat(1);
}

function step() {
  const n = pos.length, k = 1 / Math.sqrt(Math.max(n, 1));
  const t = 0.05 * heat;
  for (let i = 0; i < n; i++) {
    const a = pos[i];
    for (let j = i + 1; j < n; j++) {
      const b = pos[j], dx = a.x - b.x, dy = a.y - b.y, d2 = dx * dx + dy * dy + 1e-4;
      const f = (k * k) / d2 * 0.02;
      a.vx += dx * f; a.vy += dy * f; b.vx -= dx * f; b.vy -= dy * f;
    }
    a.vx -= a.x * 0.01; a.vy -= a.y * 0.01;
  }
  for (const [i, j] of graph.edges) {
    const a = pos[i], b = pos[j], dx = a.x - b.x, dy = a.y - b.y;
    const d = Math.sqrt(dx * dx + dy * dy) + 1e-6, f = (d - k) * 0.05 / d;
    a.vx -= dx * f; a.vy -= dy * f; b.vx += dx * f; b.vy += dy * f;
  }
  pos.forEach((p, i) => {
    if (i === drag) { p.vx = p.vy = 0; return; }
    p.x += Math.max(-t, Math.min(t, p.vx)); p.y += Math.max(-t, Math.min(t, p.vy));
    p.vx *= 0.6; p.vy *= 0.6;
  });
  heat *= 0.985;
  if (heat < 0.02) heat = 0;
}

// --- Rendering -------------------------------------------------------------
let view = null, hover = -1, drag = -1;
const PAD = 48;

function fitTarget(w, h) {
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const p of pos) {
    if (p.x < minX) minX = p.x; if (p.x > maxX) maxX = p.x;
    if (p.y < minY) minY = p.y; if (p.y > maxY) maxY = p.y;
  }
  const s = Math.min((w - 2 * PAD) / (maxX - minX || 1), (h - 2 * PAD) / (maxY - minY || 1));
  return { s, cx: (minX + maxX) / 2, cy: (minY + maxY) / 2 };
}

const X = (p) => (p.x - view.cx) * view.s + view.w / 2;
const Y = (p) => (p.y - view.cy) * view.s + view.h / 2;

// Node size follows the typical spacing on screen, so dense graphs and small
// screens get smaller nodes instead of overlapping ones.
function radius() {
  const spacing = view ? view.s / Math.sqrt(Math.max(pos.length, 1)) : 40;
  return Math.max(3, Math.min(15, spacing * 0.32));
}

function draw() {
  requestAnimationFrame(draw);
  if (heat > 0) step();
  const dpr = devicePixelRatio, w = canvas.clientWidth, h = canvas.clientHeight;
  if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
    canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr); dirty = true;
  }
  if (!dirty && heat === 0) return;
  dirty = false;

  // Ease the camera toward the bounding box, but hold it still while dragging.
  if (pos.length && drag < 0) {
    const t = fitTarget(w, h);
    if (!view || view.w !== w || view.h !== h) view = { ...t, w, h };
    else {
      const e = 0.15;
      view.s += (t.s - view.s) * e; view.cx += (t.cx - view.cx) * e; view.cy += (t.cy - view.cy) * e;
      if (Math.abs(t.s - view.s) / t.s > 1e-3 || Math.abs(t.cx - view.cx) * t.s > 0.5 || Math.abs(t.cy - view.cy) * t.s > 0.5) dirty = true;
    }
  }

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  if (!pos.length) {
    ctx.fillStyle = "#5d627a"; ctx.font = "14px system-ui"; ctx.textAlign = "center";
    ctx.fillText("Enter some edges to get started", w / 2, h / 2);
    return;
  }

  const solved = solution.size > 0;
  const focus = hover >= 0 ? new Set([hover, ...adj[hover]]) : null;
  const r = radius();

  // Edges
  ctx.lineCap = "round";
  for (const [i, j] of graph.edges) {
    const a = pos[i], b = pos[j];
    const lit = focus ? (i === hover || j === hover) : false;
    const dom = solved && (solution.has(i) || solution.has(j));
    let color = "rgba(130, 140, 185, 0.28)";
    if (dom) color = "rgba(255, 122, 61, 0.55)";
    if (focus && !lit) color = "rgba(130, 140, 185, 0.08)";
    if (lit) color = "rgba(255, 210, 180, 0.95)";
    ctx.strokeStyle = color;
    ctx.lineWidth = lit ? 2.2 : dom ? 1.8 : 1.2;
    ctx.beginPath(); ctx.moveTo(X(a), Y(a)); ctx.lineTo(X(b), Y(b)); ctx.stroke();
  }

  // Nodes
  ctx.font = `600 ${Math.max(10, r * 0.85)}px Inter, system-ui, sans-serif`;
  ctx.textAlign = "center"; ctx.textBaseline = "middle";
  pos.forEach((p, i) => {
    const x = X(p), y = Y(p), inSet = solution.has(i);
    const faded = focus && !focus.has(i);
    const rr = i === hover ? r * 1.2 : r;
    ctx.globalAlpha = faded ? 0.3 : 1;
    if (inSet) {
      const g = ctx.createLinearGradient(x - rr, y - rr, x + rr, y + rr);
      g.addColorStop(0, "#ff8a4c"); g.addColorStop(1, "#ff3d81");
      ctx.shadowColor = "rgba(255, 80, 100, 0.85)"; ctx.shadowBlur = rr * 1.4;
      ctx.fillStyle = g;
      ctx.beginPath(); ctx.arc(x, y, rr, 0, 2 * Math.PI); ctx.fill();
      ctx.shadowBlur = 0;
      ctx.strokeStyle = "rgba(255, 230, 215, 0.7)"; ctx.lineWidth = 1.2; ctx.stroke();
    } else {
      ctx.fillStyle = solved ? "#251d2b" : "#171a28";
      ctx.strokeStyle = i === hover ? "#ffd2bd" : solved ? "#ff9a6b" : "#6c7393";
      ctx.lineWidth = 1.6;
      ctx.beginPath(); ctx.arc(x, y, rr, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
    }
    if (r >= 10) {
      ctx.fillStyle = inSet ? "#fff" : "#cfd2e6";
      ctx.fillText(graph.names[i], x, y + 0.5);
    }
  });
  ctx.globalAlpha = 1;

  // Label the hovered vertex when nodes are too small to carry text.
  if (hover >= 0 && r < 10) {
    const p = pos[hover], label = graph.names[hover];
    ctx.font = "600 12px Inter, system-ui, sans-serif";
    const tw = ctx.measureText(label).width + 14, x = X(p), y = Y(p) - r - 16;
    ctx.fillStyle = "rgba(15, 17, 27, 0.9)"; ctx.strokeStyle = "rgba(255,255,255,0.15)";
    ctx.beginPath(); ctx.roundRect(x - tw / 2, y - 11, tw, 22, 6); ctx.fill(); ctx.stroke();
    ctx.fillStyle = "#e8e9f1"; ctx.fillText(label, x, y);
  }
}
requestAnimationFrame(draw);

// --- Interaction: hover and drag -------------------------------------------
function pick(e) {
  if (!view) return -1;
  const rect = canvas.getBoundingClientRect(), mx = e.clientX - rect.left, my = e.clientY - rect.top;
  const r = Math.max(radius(), 8) + 3;
  let best = -1, bd = r * r;
  pos.forEach((p, i) => {
    const dx = X(p) - mx, dy = Y(p) - my, d = dx * dx + dy * dy;
    if (d < bd) { bd = d; best = i; }
  });
  return best;
}

function updateInfo() {
  const info = $("info");
  if (hover < 0) { info.hidden = true; return; }
  info.hidden = false;
  const tag = solution.has(hover) ? " · in set" : "";
  info.textContent = `${graph.names[hover]} · deg ${adj[hover].length}${tag}`;
}

canvas.addEventListener("pointermove", (e) => {
  if (drag >= 0) {
    const rect = canvas.getBoundingClientRect();
    pos[drag].x = (e.clientX - rect.left - view.w / 2) / view.s + view.cx;
    pos[drag].y = (e.clientY - rect.top - view.h / 2) / view.s + view.cy;
    reheat(0.5);
    return;
  }
  const h = pick(e);
  if (h !== hover) { hover = h; dirty = true; updateInfo(); }
  canvas.style.cursor = h >= 0 ? "grab" : "default";
});
canvas.addEventListener("pointerdown", (e) => {
  const h = pick(e);
  if (h < 0) return;
  drag = hover = h;
  canvas.setPointerCapture(e.pointerId);
  canvas.style.cursor = "grabbing";
});
const endDrag = () => { if (drag >= 0) { drag = -1; canvas.style.cursor = "grab"; reheat(0.3); } };
canvas.addEventListener("pointerup", endDrag);
canvas.addEventListener("pointercancel", endDrag);
canvas.addEventListener("pointerleave", () => { if (drag < 0 && hover >= 0) { hover = -1; dirty = true; updateInfo(); } });
updateInfo();

$("relayout").onclick = () => {
  for (const p of pos) { p.x = Math.random() - 0.5; p.y = Math.random() - 0.5; p.vx = p.vy = 0; }
  reheat(1);
};

// --- Solving ---------------------------------------------------------------
// Workers share memory only when the page is cross-origin isolated (served
// with COOP/COEP headers, see _headers and serve.py). Otherwise use one.
const THREADS = crossOriginIsolated ? navigator.hardwareConcurrency || 4 : 1;
$("threads").textContent = `${THREADS} thread${THREADS > 1 ? "s" : ""}`;
let workers = [];

function setStatus(text, { busy = false, err = false } = {}) {
  status.className = err ? "err" : "";
  status.innerHTML = "";
  if (busy) status.append(Object.assign(document.createElement("span"), { className: "spinner" }));
  status.append(text);
}

function stopWorkers() {
  workers.forEach((w) => w.terminate());
  workers = [];
  $("solve").disabled = false;
  $("solve").classList.remove("busy");
  $("solve").textContent = "Solve";
  $("cancel").disabled = true;
}

function showResult(set, ms) {
  $("gamma").textContent = set.length;
  $("sv").textContent = graph.names.length;
  $("se").textContent = graph.edges.length;
  $("st").textContent = ms < 1000 ? `${ms.toFixed(ms < 10 ? 1 : 0)}ms` : `${(ms / 1000).toFixed(2)}s`;
  const list = $("setlist");
  list.replaceChildren(...set.map((v) => Object.assign(document.createElement("span"), { className: "chip", textContent: graph.names[v] })));
  $("result").hidden = false;
}

$("solve").onclick = () => {
  stopWorkers();
  setGraph(parse(input.value));
  const n = graph.names.length, edges = graph.edges;
  if (!n) { setStatus("The graph is empty.", { err: true }); return; }
  const state = new Int32Array(new (THREADS > 1 ? SharedArrayBuffer : ArrayBuffer)(4 * (n + 1)));
  state.fill(0x7fffffff, 1);
  const start = performance.now(), results = [];
  $("solve").disabled = true;
  $("solve").classList.add("busy");
  $("solve").textContent = "Solving…";
  $("cancel").disabled = false;
  setStatus(`Searching ${n} vertices, ${edges.length} edges on ${THREADS} thread${THREADS > 1 ? "s" : ""}`, { busy: true });
  for (let i = 0; i < THREADS; i++) {
    const worker = new Worker("worker.js");
    worker.onmessage = ({ data }) => {
      if (data.error) { stopWorkers(); setStatus("Error: " + data.error, { err: true }); return; }
      results.push(data.sets);
      if (results.length < THREADS) return;
      // Best set per connected component over all workers.
      const set = results[0].flatMap((_, c) =>
        results.map((r) => r[c]).reduce((a, b) => (b.length < a.length ? b : a)));
      set.sort((a, b) => a - b);
      const ms = performance.now() - start;
      stopWorkers();
      solution = new Set(set);
      dirty = true;
      showResult(set, ms);
      setStatus("");
      updateInfo();
    };
    worker.onerror = (e) => { stopWorkers(); setStatus("Worker error: " + e.message, { err: true }); };
    worker.postMessage({ n, edges, state: state.buffer, splitTarget: THREADS > 1 ? 64 * THREADS : 1 });
    workers.push(worker);
  }
};

$("cancel").onclick = () => { stopWorkers(); setStatus("Cancelled."); };

for (const id of Object.keys(EXAMPLES)) {
  $(id).onclick = () => { input.value = EXAMPLES[id]; setGraph(parse(input.value)); };
}

$("rand").onclick = () => {
  const n = +$("rn").value, p = +$("rp").value, lines = [];
  for (let i = 0; i < n; i++) lines.push(String(i));
  for (let i = 0; i < n; i++)
    for (let j = i + 1; j < n; j++) if (Math.random() < p) lines.push(`${i} ${j}`);
  input.value = lines.join("\n") + "\n";
  setGraph(parse(input.value));
};

// Ctrl/Cmd+Enter solves from anywhere.
addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && !$("solve").disabled) $("solve").click();
});

input.oninput = () => setGraph(parse(input.value));
input.value = EXAMPLES.ex1;
setGraph(parse(input.value));
