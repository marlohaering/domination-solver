//! Plain C-ABI exports for WebAssembly. No wasm-bindgen needed: JavaScript
//! copies the edge list into linear memory and reads the result back.
//!
//! Protocol:
//! 1. `ptr = alloc(2 * m)` and write the edges as `[a0, b0, a1, b1, ...]` (u32).
//! 2. `res = solve(n, ptr, m)`; `res` points to `[k, v0, ..., v(k-1)]` (u32).
//! 3. Free both buffers with `dealloc(ptr, 2 * m)` and `dealloc(res, k + 1)`.

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

/// Solves the instance and returns a pointer to `[k, v0, ..., v(k-1)]`.
///
/// # Safety
/// `edges` must point to `2 * m` readable u32 values.
#[no_mangle]
pub unsafe extern "C" fn solve(n: usize, edges: *const u32, m: usize) -> *mut u32 {
    let flat = std::slice::from_raw_parts(edges, 2 * m);
    let edges: Vec<(usize, usize)> = flat
        .chunks_exact(2)
        .map(|e| (e[0] as usize, e[1] as usize))
        .collect();
    let set = crate::minimum_dominating_set(n, &edges);
    let mut out = Vec::with_capacity(set.len() + 1);
    out.push(set.len() as u32);
    out.extend(set.iter().map(|&v| v as u32));
    let mut out = out.into_boxed_slice();
    let ptr = out.as_mut_ptr();
    std::mem::forget(out);
    ptr
}
