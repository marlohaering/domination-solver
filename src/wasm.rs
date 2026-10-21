//! Plain C-ABI exports for WebAssembly. No wasm-bindgen needed: JavaScript
//! copies the edge list into linear memory and reads the result back.
//!
//! Protocol:
//! 1. `ptr = alloc(2 * m)` and write the edges as `[a0, b0, a1, b1, ...]` (u32).
//! 2. `res = solve(n, ptr, m, split_target)`; `res` points to
//!    `[len, components, k0, v.., k1, v.., ...]` (u32): this worker's best
//!    set per connected component.
//! 3. Free both buffers with `dealloc(ptr, 2 * m)` and `dealloc(res, len)`.
//!
//! Several Web Workers can search the same instance in parallel. They
//! coordinate through the imported `env.claim`, `env.best` and `env.offer`
//! functions, which JavaScript implements on a SharedArrayBuffer.

use crate::{Instance, Shared};

#[link(wasm_import_module = "env")]
extern "C" {
    fn claim() -> u32;
    fn best(comp: u32) -> u32;
    fn offer(comp: u32, len: u32);
}

struct JsShared;

impl Shared for JsShared {
    fn claim(&self) -> usize {
        unsafe { claim() as usize }
    }
    fn best(&self, comp: usize) -> usize {
        unsafe { best(comp as u32) as usize }
    }
    fn offer(&self, comp: usize, len: usize) {
        unsafe { offer(comp as u32, len as u32) }
    }
}

/// Allocates space for `len` u32 values.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u32 {
    let mut buf = vec![0u32; len].into_boxed_slice();
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Frees a buffer returned by `alloc` or `solve`.
///
/// # Safety
/// `ptr` and `len` must come from a previous `alloc(len)` or `solve` call.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u32, len: usize) {
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
}

/// Runs one worker on the instance (see the module docs for the format).
///
/// # Safety
/// `edges` must point to `2 * m` readable u32 values.
#[no_mangle]
pub unsafe extern "C" fn solve(n: usize, edges: *const u32, m: usize, split_target: usize) -> *mut u32 {
    let flat = std::slice::from_raw_parts(edges, 2 * m);
    let edges: Vec<(usize, usize)> = flat
        .chunks_exact(2)
        .map(|e| (e[0] as usize, e[1] as usize))
        .collect();
    let sets = Instance::new(n, &edges).search(&JsShared, split_target);
    let mut out = vec![0, sets.len() as u32];
    for set in &sets {
        out.push(set.len() as u32);
        out.extend(set.iter().map(|&v| v as u32));
    }
    out[0] = out.len() as u32;
    let mut out = out.into_boxed_slice();
    let ptr = out.as_mut_ptr();
    std::mem::forget(out);
    ptr
}
