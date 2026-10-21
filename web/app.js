const $ = (id) => document.getElementById(id);
const input = $("input"), status = $("status"), canvas = $("c");

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

// --- Drawing: a small force-directed layout -------------------------------
let graph = { names: [], edges: [] }, pos = [], solution = new Set(), frames = 0;

function setGraph(g) {
  const old = new Map(graph.names.map((name, i) => [name, pos[i]]));
  graph = g;
  pos = g.names.map((name) => old.get(name) ?? { x: Math.random() - 0.5, y: Math.random() - 0.5, vx: 0, vy: 0 });
  solution = new Set();
  frames = 300;
}

function step() {
  const n = pos.length, k = 1 / Math.sqrt(Math.max(n, 1));
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
  for (const p of pos) {
    p.x += Math.max(-0.05, Math.min(0.05, p.vx)); p.y += Math.max(-0.05, Math.min(0.05, p.vy));
    p.vx *= 0.6; p.vy *= 0.6;
  }
}

function draw() {
  if (frames > 0) { step(); frames--; }
  const dpr = devicePixelRatio, w = canvas.clientWidth, h = canvas.clientHeight;
  canvas.width = w * dpr; canvas.height = h * dpr;
  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);
  if (pos.length) {
    const xs = pos.map((p) => p.x), ys = pos.map((p) => p.y);
    const minX = Math.min(...xs), maxX = Math.max(...xs), minY = Math.min(...ys), maxY = Math.max(...ys);
    const s = Math.min((w - 60) / (maxX - minX || 1), (h - 60) / (maxY - minY || 1));
    const X = (p) => 30 + (p.x - minX) * s + (w - 60 - (maxX - minX) * s) / 2;
    const Y = (p) => 30 + (p.y - minY) * s + (h - 60 - (maxY - minY) * s) / 2;
    ctx.lineWidth = 1.5;
    for (const [i, j] of graph.edges) {
      const dom = solution.has(i) || solution.has(j);
      ctx.strokeStyle = solution.size && dom ? "#e08a3c" : "#bbb";
      ctx.beginPath(); ctx.moveTo(X(pos[i]), Y(pos[i])); ctx.lineTo(X(pos[j]), Y(pos[j])); ctx.stroke();
    }
    const r = pos.length > 100 ? 6 : 12;
    ctx.font = `${r}px system-ui`; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    pos.forEach((p, i) => {
      const inSet = solution.has(i);
      ctx.fillStyle = inSet ? "#d9480f" : "#fff";
      ctx.strokeStyle = inSet ? "#d9480f" : "#555";
      ctx.beginPath(); ctx.arc(X(p), Y(p), r, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
      if (r >= 10) { ctx.fillStyle = inSet ? "#fff" : "#222"; ctx.fillText(graph.names[i], X(p), Y(p)); }
    });
  }
  requestAnimationFrame(draw);
}
requestAnimationFrame(draw);

// --- Solving ---------------------------------------------------------------
let worker = null;

function stopWorker() {
  worker?.terminate();
  worker = null;
  $("solve").disabled = false;
  $("cancel").disabled = true;
}

$("solve").onclick = () => {
  stopWorker();
  setGraph(parse(input.value));
  worker = new Worker("worker.js");
  $("solve").disabled = true;
  $("cancel").disabled = false;
  status.textContent = `Solving (${graph.names.length} vertices, ${graph.edges.length} edges)...`;
  worker.onmessage = ({ data }) => {
    stopWorker();
    if (data.error) { status.textContent = "Error: " + data.error; return; }
    solution = new Set(data.set);
    status.textContent =
      `Domination number: ${data.set.length}\n` +
      `Set: ${data.set.map((v) => graph.names[v]).join(" ")}\n` +
      `Time: ${data.ms.toFixed(1)} ms`;
  };
  worker.onerror = (e) => { stopWorker(); status.textContent = "Worker error: " + e.message; };
  worker.postMessage({ n: graph.names.length, edges: graph.edges });
};

$("cancel").onclick = () => { stopWorker(); status.textContent = "Cancelled."; };

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

input.oninput = () => setGraph(parse(input.value));
input.value = EXAMPLES.ex1;
setGraph(parse(input.value));
