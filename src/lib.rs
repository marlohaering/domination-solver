//! Exact minimum dominating set solver (branch and bound).
//!
//! A set `D` dominates a graph if every vertex is in `D` or adjacent to a
//! vertex in `D`. Vertex sets are stored as bitsets (`Vec<u64>`), so set union,
//! difference and counting are a handful of word operations.
//!
//! Connected components are solved independently. Within a component, the top
//! of the search tree is split into tasks that several workers (threads, or
//! Web Workers in the browser) claim from a shared counter. Workers also share
//! the size of the best solution found so far, so each one prunes with the
//! bounds found by all of them.

#[cfg(target_arch = "wasm32")]
pub mod wasm;

use std::sync::atomic::{AtomicUsize, Ordering};

type Bits = Vec<u64>;

fn words(n: usize) -> usize {
    (n + 63) / 64
}

fn set(b: &mut [u64], i: usize) {
    b[i / 64] |= 1 << (i % 64);
}

fn get(b: &[u64], i: usize) -> bool {
    b[i / 64] >> (i % 64) & 1 == 1
}

fn count(b: &[u64]) -> usize {
    b.iter().map(|w| w.count_ones() as usize).sum()
}

/// Number of bits set in `a & b`.
fn count_and(a: &[u64], b: &[u64]) -> usize {
    a.iter().zip(b).map(|(a, b)| (a & b).count_ones() as usize).sum()
}

/// Number of bits set in `a & !b`.
fn count_and_not(a: &[u64], b: &[u64]) -> usize {
    a.iter().zip(b).map(|(a, b)| (a & !b).count_ones() as usize).sum()
}

/// Whether `a & mask` is a subset of `b`.
fn subset_within(a: &[u64], mask: &[u64], b: &[u64]) -> bool {
    a.iter().zip(mask).zip(b).all(|((a, m), b)| a & m & !b == 0)
}

fn or(a: &[u64], b: &[u64]) -> Bits {
    a.iter().zip(b).map(|(a, b)| a | b).collect()
}

fn iter_bits(b: &[u64]) -> impl Iterator<Item = usize> + '_ {
    b.iter().enumerate().flat_map(|(wi, &w)| {
        let mut w = w;
        std::iter::from_fn(move || {
            if w == 0 {
                return None;
            }
            let bit = w.trailing_zeros() as usize;
            w &= w - 1;
            Some(wi * 64 + bit)
        })
    })
}

/// State shared between workers searching the same instance.
pub trait Shared: Sync {
    /// Claims the next task id. Ids are handed out in increasing order.
    fn claim(&self) -> usize;
    /// Size of the best solution of component `comp` found by any worker.
    fn best(&self, comp: usize) -> usize;
    /// Reports a solution of size `len` for component `comp`.
    fn offer(&self, comp: usize, len: usize);
}

/// [`Shared`] implementation for threads in one process.
pub struct AtomicShared {
    next: AtomicUsize,
    best: Vec<AtomicUsize>,
}

impl AtomicShared {
    pub fn new(components: usize) -> AtomicShared {
        AtomicShared {
            next: AtomicUsize::new(0),
            best: (0..components).map(|_| AtomicUsize::new(usize::MAX)).collect(),
        }
    }
}

impl Shared for AtomicShared {
    fn claim(&self) -> usize {
        self.next.fetch_add(1, Ordering::Relaxed)
    }
    fn best(&self, comp: usize) -> usize {
        self.best[comp].load(Ordering::Relaxed)
    }
    fn offer(&self, comp: usize, len: usize) {
        self.best[comp].fetch_min(len, Ordering::Relaxed);
    }
}

/// A connected component with its own local vertex numbering.
struct Comp {
    /// Closed neighbourhood N[v] of every vertex.
    ball: Vec<Bits>,
    all: Bits,
    /// Local vertex index -> vertex index in the input graph.
    global: Vec<usize>,
}

/// A search node: vertices chosen so far, the vertices they dominate, and
/// vertices that may no longer be chosen.
#[derive(Clone)]
struct Node {
    chosen: Vec<usize>,
    covered: Bits,
    excluded: Bits,
}

enum Expansion {
    /// All vertices are dominated.
    Done,
    /// Some vertex can no longer be dominated.
    Dead,
    Branch(Branch),
}

struct Branch {
    uncovered: Bits,
    n_uncovered: usize,
    /// Uncovered vertices, sorted by their number of candidate dominators.
    by_cands: Vec<(usize, usize)>,
    /// Vertices to branch on, in order (filled by `choose_candidates`).
    cands: Vec<usize>,
    /// Candidates that are never needed (another candidate covers a superset).
    redundant: Vec<usize>,
}

impl Comp {
    fn new(global: Vec<usize>, ball: Vec<Bits>) -> Comp {
        let n = global.len();
        let mut all = vec![!0u64; words(n)];
        if n % 64 != 0 {
            *all.last_mut().unwrap() = (1 << (n % 64)) - 1;
        }
        Comp { ball, all, global }
    }

    fn root(&self) -> Node {
        let w = self.all.len();
        Node { chosen: Vec::new(), covered: vec![0; w], excluded: vec![0; w] }
    }

    /// Greedy upper bound: repeatedly take the vertex covering the most
    /// uncovered vertices.
    fn greedy(&self) -> Vec<usize> {
        let n = self.global.len();
        let mut covered = vec![0u64; self.all.len()];
        let mut result = Vec::new();
        while count(&covered) < n {
            let v = (0..n)
                .max_by_key(|&v| count_and_not(&self.ball[v], &covered))
                .unwrap();
            result.push(v);
            covered = or(&covered, &self.ball[v]);
        }
        result
    }

    /// Finds the uncovered vertices and their number of candidate
    /// dominators (N[u] \ excluded).
    fn expand(&self, node: &Node) -> Expansion {
        let uncovered: Bits = self.all.iter().zip(&node.covered).map(|(a, c)| a & !c).collect();
        let n_uncovered = count(&uncovered);
        if n_uncovered == 0 {
            return Expansion::Done;
        }
        let mut by_cands = Vec::with_capacity(n_uncovered);
        for u in iter_bits(&uncovered) {
            let k = count_and_not(&self.ball[u], &node.excluded);
            if k == 0 {
                return Expansion::Dead;
            }
            by_cands.push((k, u));
        }
        by_cands.sort_unstable();
        Expansion::Branch(Branch { uncovered, n_uncovered, by_cands, cands: Vec::new(), redundant: Vec::new() })
    }

    /// Lists the candidates for the uncovered vertex with the fewest
    /// candidate dominators.
    fn choose_candidates(&self, node: &Node, b: &mut Branch) {
        // Best coverage first. A candidate whose newly covered vertices are a
        // subset of an earlier candidate's can be dropped: swapping it for the
        // earlier one never makes a solution worse.
        let mut sorted: Vec<(usize, usize)> = iter_bits(&self.ball[b.by_cands[0].1])
            .filter(|&v| !get(&node.excluded, v))
            .map(|v| (count_and(&self.ball[v], &b.uncovered), v))
            .collect();
        sorted.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (i, &(_, v)) in sorted.iter().enumerate() {
            let dominated = sorted[..i]
                .iter()
                .any(|&(_, w)| subset_within(&self.ball[v], &b.uncovered, &self.ball[w]));
            if dominated {
                b.redundant.push(v);
            } else {
                b.cands.push(v);
            }
        }
    }

    /// Children of `node`: one per candidate. Candidates tried earlier (and
    /// redundant ones) are excluded in later children.
    fn children<'a>(&'a self, node: &'a Node, b: &'a Branch) -> impl Iterator<Item = Node> + 'a {
        let mut excluded = node.excluded.clone();
        for &v in &b.redundant {
            set(&mut excluded, v);
        }
        b.cands.iter().map(move |&v| {
            let mut chosen = node.chosen.clone();
            chosen.push(v);
            let child = Node { chosen, covered: or(&node.covered, &self.ball[v]), excluded: excluded.clone() };
            set(&mut excluded, v);
            child
        })
    }

    /// Whether `budget` more vertices could possibly dominate the rest
    /// (false means the node can be pruned).
    fn can_finish(&self, node: &Node, b: &Branch, budget: usize) -> bool {
        // Uncovered vertices with pairwise disjoint candidate sets each need
        // their own dominator.
        let mut used = vec![0u64; self.all.len()];
        let mut packing = 0;
        for &(_, u) in &b.by_cands {
            let ball = &self.ball[u];
            let disjoint = ball.iter().zip(&node.excluded).zip(&used).all(|((b, e), used)| b & !e & used == 0);
            if disjoint {
                packing += 1;
                if packing > budget {
                    return false;
                }
                for ((used, b), e) in used.iter_mut().zip(ball).zip(&node.excluded) {
                    *used |= b & !e;
                }
            }
        }
        // Even the `budget` vertices with the largest gains must cover
        // everything.
        let mut gains: Vec<usize> = (0..self.global.len())
            .filter(|&v| !get(&node.excluded, v))
            .map(|v| count_and(&self.ball[v], &b.uncovered))
            .filter(|&g| g > 0)
            .collect();
        if gains.len() > budget {
            gains.select_nth_unstable_by(budget - 1, |a, b| b.cmp(a));
            gains.truncate(budget);
        }
        gains.iter().sum::<usize>() >= b.n_uncovered
    }

    /// Visits the nodes at depth `max_depth` (and leaves above it) in a fixed
    /// order that does not depend on bounds, so all workers agree on task ids.
    /// Stops early when `f` returns false.
    fn walk(&self, node: &Node, depth: usize, max_depth: usize, f: &mut dyn FnMut(&Node) -> bool) -> bool {
        if depth == max_depth {
            return f(node);
        }
        match self.expand(node) {
            Expansion::Dead => true,
            Expansion::Done => f(node),
            Expansion::Branch(mut b) => {
                self.choose_candidates(node, &mut b);
                self.children(node, &b).all(|child| self.walk(&child, depth + 1, max_depth, f))
            }
        }
    }

    /// Smallest split depth that yields at least `target` tasks.
    fn split_depth(&self, target: usize) -> usize {
        let mut last = 0;
        for depth in 1..=8 {
            let mut tasks = 0;
            self.walk(&self.root(), 0, depth, &mut |_| {
                tasks += 1;
                tasks < target
            });
            if tasks >= target || tasks == last {
                return depth;
            }
            last = tasks;
        }
        8
    }
}

struct Searcher<'a> {
    comp: &'a Comp,
    id: usize,
    shared: &'a dyn Shared,
    best: Vec<usize>,
}

impl Searcher<'_> {
    fn search(&mut self, node: &Node) {
        let best = self.shared.best(self.id);
        if node.covered == self.comp.all {
            if node.chosen.len() < best {
                self.best = node.chosen.clone();
                self.shared.offer(self.id, node.chosen.len());
            }
            return;
        }
        // Every further vertex must strictly improve on the best solution.
        if node.chosen.len() + 1 >= best {
            return;
        }
        match self.comp.expand(node) {
            Expansion::Dead | Expansion::Done => {}
            Expansion::Branch(mut b) => {
                let budget = best - 1 - node.chosen.len();
                if !self.comp.can_finish(node, &b, budget) {
                    return;
                }
                self.comp.choose_candidates(node, &mut b);
                for child in self.comp.children(node, &b) {
                    self.search(&child);
                    if node.chosen.len() + 1 >= self.shared.best(self.id) {
                        return;
                    }
                }
            }
        }
    }
}

/// A graph split into connected components, ready to be searched.
pub struct Instance {
    comps: Vec<Comp>,
}

impl Instance {
    /// Builds an instance for a graph with `n` vertices `0..n`.
    pub fn new(n: usize, edges: &[(usize, usize)]) -> Instance {
        let mut adj = vec![Vec::new(); n];
        for &(a, b) in edges {
            assert!(a < n && b < n, "edge ({}, {}) out of range", a, b);
            adj[a].push(b);
            adj[b].push(a);
        }
        let mut local = vec![usize::MAX; n];
        let mut comps = Vec::new();
        for start in 0..n {
            if local[start] != usize::MAX {
                continue;
            }
            let mut global = vec![start];
            local[start] = 0;
            let mut i = 0;
            while i < global.len() {
                for &v in &adj[global[i]] {
                    if local[v] == usize::MAX {
                        local[v] = global.len();
                        global.push(v);
                    }
                }
                i += 1;
            }
            // Keep the input order within the component.
            global.sort_unstable();
            for (i, &v) in global.iter().enumerate() {
                local[v] = i;
            }
            let mut ball = vec![vec![0u64; words(global.len())]; global.len()];
            for (i, b) in ball.iter_mut().enumerate() {
                set(b, i);
                for &v in &adj[global[i]] {
                    set(b, local[v]);
                }
            }
            comps.push(Comp::new(global, ball));
        }
        Instance { comps }
    }

    pub fn components(&self) -> usize {
        self.comps.len()
    }

    /// Runs one worker. With `split_target > 1` each component's search tree
    /// is cut into roughly that many tasks, claimed through `shared`.
    /// Returns this worker's best set per component (input vertex indices).
    pub fn search(&self, shared: &dyn Shared, split_target: usize) -> Vec<Vec<usize>> {
        let mut task = 0;
        let mut mine = shared.claim();
        let mut result = Vec::new();
        for (id, comp) in self.comps.iter().enumerate() {
            let mut s = Searcher { comp, id, shared, best: comp.greedy() };
            shared.offer(id, s.best.len());
            let depth = if split_target > 1 { comp.split_depth(split_target) } else { 0 };
            comp.walk(&comp.root(), 0, depth, &mut |node| {
                if task == mine {
                    s.search(node);
                    mine = shared.claim();
                }
                task += 1;
                true
            });
            result.push(s.best.iter().map(|&v| comp.global[v]).collect());
        }
        result
    }
}

/// Combines the per-component results of several workers into one set.
pub fn merge(results: &[Vec<Vec<usize>>]) -> Vec<usize> {
    let components = results.first().map_or(0, |r| r.len());
    let mut set: Vec<usize> = (0..components)
        .flat_map(|c| results.iter().map(|r| &r[c]).min_by_key(|s| s.len()).unwrap().iter().copied())
        .collect();
    set.sort_unstable();
    set
}

/// Minimum dominating set of a graph with `n` vertices, single threaded.
pub fn minimum_dominating_set(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let instance = Instance::new(n, edges);
    let shared = AtomicShared::new(instance.components());
    merge(&[instance.search(&shared, 1)])
}

/// Minimum dominating set using `threads` threads.
#[cfg(not(target_arch = "wasm32"))]
pub fn minimum_dominating_set_parallel(n: usize, edges: &[(usize, usize)], threads: usize) -> Vec<usize> {
    if threads <= 1 {
        return minimum_dominating_set(n, edges);
    }
    let instance = Instance::new(n, edges);
    let shared = AtomicShared::new(instance.components());
    let results: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|_| scope.spawn(|| instance.search(&shared, 64 * threads)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    merge(&results)
}

/// Checks whether `set` dominates the graph.
pub fn is_dominating(n: usize, edges: &[(usize, usize)], set: &[usize]) -> bool {
    let mut covered = vec![false; n];
    for &v in set {
        covered[v] = true;
    }
    for &(a, b) in edges {
        if set.contains(&a) {
            covered[b] = true;
        }
        if set.contains(&b) {
            covered[a] = true;
        }
    }
    covered.iter().all(|&c| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute force over all subsets, for small graphs.
    fn brute_force(n: usize, edges: &[(usize, usize)]) -> usize {
        (0u32..1 << n)
            .filter(|mask| {
                let set: Vec<usize> = (0..n).filter(|i| mask >> i & 1 == 1).collect();
                is_dominating(n, edges, &set)
            })
            .map(|mask| mask.count_ones() as usize)
            .min()
            .unwrap()
    }

    fn random_graphs(count: usize, max_n: u64) -> Vec<(usize, Vec<(usize, usize)>)> {
        // Small deterministic PRNG so the tests need no dependencies.
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut rand = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        (0..count)
            .map(|_| {
                let n = (rand() % max_n) as usize + 1;
                let p = rand() % 100;
                let mut edges = Vec::new();
                for a in 0..n {
                    for b in a + 1..n {
                        if rand() % 100 < p {
                            edges.push((a, b));
                        }
                    }
                }
                (n, edges)
            })
            .collect()
    }

    #[test]
    fn small_graphs() {
        let g1 = [
            (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (2, 5),
            (2, 9), (3, 4), (3, 6), (5, 8), (7, 8), (1, 7),
        ];
        let s = minimum_dominating_set(10, &g1);
        assert!(is_dominating(10, &g1, &s));
        assert_eq!(s.len(), 3);

        let g2 = [(0, 1), (0, 2)];
        assert_eq!(minimum_dominating_set(8, &g2).len(), 6);
        assert_eq!(minimum_dominating_set(0, &[]).len(), 0);
        assert_eq!(minimum_dominating_set_parallel(0, &[], 4).len(), 0);
    }

    #[test]
    fn random_graphs_match_brute_force() {
        for (n, edges) in random_graphs(400, 16) {
            let s = minimum_dominating_set(n, &edges);
            assert!(is_dominating(n, &edges, &s));
            assert_eq!(s.len(), brute_force(n, &edges), "n={} edges={:?}", n, edges);
        }
    }

    #[test]
    fn parallel_matches_sequential() {
        for (n, edges) in random_graphs(150, 40) {
            let s = minimum_dominating_set(n, &edges);
            for threads in [2, 5] {
                let p = minimum_dominating_set_parallel(n, &edges, threads);
                assert!(is_dominating(n, &edges, &p));
                assert_eq!(p.len(), s.len(), "n={} edges={:?}", n, edges);
            }
        }
    }
}
