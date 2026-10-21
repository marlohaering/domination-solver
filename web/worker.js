// One search worker. Several of these run in parallel, each with its own copy
// of the WebAssembly module, coordinating through a shared Int32Array:
//   [0]     next task id
//   [1 + c] size of the best solution found for connected component c
const module = WebAssembly.compileStreaming(fetch("domination_solver.wasm"));

onmessage = async ({ data: { n, edges, state, splitTarget } }) => {
  try {
    const shared = new Int32Array(state);
    const env = {
      claim: () => Atomics.add(shared, 0, 1),
      best: (c) => Atomics.load(shared, 1 + c),
      offer: (c, len) => {
        let cur = Atomics.load(shared, 1 + c);
        while (len < cur) {
          const prev = Atomics.compareExchange(shared, 1 + c, cur, len);
          if (prev === cur) break;
          cur = prev;
        }
      },
    };
    const { exports } = await WebAssembly.instantiate(await module, { env });
    const { memory, alloc, dealloc, solve } = exports;
    const flat = edges.flat();
    const ptr = alloc(flat.length);
    new Uint32Array(memory.buffer, ptr, flat.length).set(flat);
    const res = solve(n, ptr, edges.length, splitTarget);
    // Memory may have grown during solve(), so create views afterwards.
    const len = new Uint32Array(memory.buffer, res, 1)[0];
    const out = Array.from(new Uint32Array(memory.buffer, res, len));
    dealloc(ptr, flat.length);
    dealloc(res, len);
    // out = [len, components, k0, v.., k1, v.., ...]
    const sets = [];
    for (let i = 2; i < out.length; i += 1 + out[i]) sets.push(out.slice(i + 1, i + 1 + out[i]));
    postMessage({ sets });
  } catch (e) {
    postMessage({ error: String(e) });
  }
};
