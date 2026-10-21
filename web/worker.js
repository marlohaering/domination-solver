// Runs the WebAssembly solver off the main thread so the page stays responsive.
const wasm = WebAssembly.instantiateStreaming(fetch("domination_solver.wasm"))
  .then(({ instance }) => instance.exports);

onmessage = async ({ data: { n, edges } }) => {
  try {
    const { memory, alloc, dealloc, solve } = await wasm;
    const flat = edges.flat();
    const ptr = alloc(flat.length);
    new Uint32Array(memory.buffer, ptr, flat.length).set(flat);
    const start = performance.now();
    const res = solve(n, ptr, edges.length);
    const ms = performance.now() - start;
    // Memory may have grown during solve(), so create views afterwards.
    const k = new Uint32Array(memory.buffer, res, 1)[0];
    const set = Array.from(new Uint32Array(memory.buffer, res + 4, k));
    dealloc(ptr, flat.length);
    dealloc(res, k + 1);
    postMessage({ set, ms });
  } catch (e) {
    postMessage({ error: String(e) });
  }
};
