//! Exact minimum dominating set solver (branch and bound).
//!
//! A set `D` dominates a graph if every vertex is in `D` or adjacent to a
//! vertex in `D`. Vertex sets are stored as bitsets (`Vec<u64>`), so set union,
//! difference and counting are a handful of word operations.

pub mod wasm;

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

/// Number of bits set in `a & b & !c`.
fn count_and_not(a: &[u64], b: &[u64], c: &[u64]) -> usize {
    a.iter()
        .zip(b)
        .zip(c)
        .map(|((a, b), c)| (a & b & !c).count_ones() as usize)
        .sum()
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

pub struct Solver {
    n: usize,
    /// Closed neighbourhood N[v] of every vertex.
    ball: Vec<Bits>,
    /// Best dominating set found so far.
    best: Vec<usize>,
    /// Search nodes visited, for statistics.
    pub nodes: u64,
}

impl Solver {
    /// Builds a solver for a graph with `n` vertices `0..n` and undirected edges.
    pub fn new(n: usize, edges: &[(usize, usize)]) -> Solver {
        let mut ball = vec![vec![0u64; words(n)]; n];
        for (v, b) in ball.iter_mut().enumerate() {
            set(b, v);
        }
        for &(a, b) in edges {
            assert!(a < n && b < n, "edge ({}, {}) out of range", a, b);
            set(&mut ball[a], b);
            set(&mut ball[b], a);
        }
        Solver {
            n,
            ball,
            best: (0..n).collect(),
            nodes: 0,
        }
    }

    /// Returns a minimum dominating set (sorted vertex indices).
    pub fn solve(mut self) -> Vec<usize> {
        let w = words(self.n);
        self.best = self.greedy();
        let mut chosen = Vec::new();
        self.search(&mut chosen, &vec![0; w], &vec![0; w]);
        self.best.sort_unstable();
        self.best
    }

    /// Greedy upper bound: repeatedly take the vertex covering the most
    /// uncovered vertices.
    fn greedy(&self) -> Vec<usize> {
        let w = words(self.n);
        let mut covered = vec![0u64; w];
        let all = self.all();
        let mut result = Vec::new();
        while count(&covered) < self.n {
            let v = (0..self.n)
                .max_by_key(|&v| count_and_not(&self.ball[v], &all, &covered))
                .unwrap();
            result.push(v);
            for (c, b) in covered.iter_mut().zip(&self.ball[v]) {
                *c |= b;
            }
        }
        result
    }

    fn all(&self) -> Bits {
        let mut all = vec![!0u64; words(self.n)];
        if self.n % 64 != 0 {
            *all.last_mut().unwrap() = (1 << (self.n % 64)) - 1;
        }
        all
    }

    /// `chosen`: vertices in the partial solution, `covered`: vertices they
    /// dominate, `excluded`: vertices that may no longer be chosen.
    fn search(&mut self, chosen: &mut Vec<usize>, covered: &Bits, excluded: &Bits) {
        self.nodes += 1;
        let uncovered: Bits = self
            .all()
            .iter()
            .zip(covered)
            .map(|(a, c)| a & !c)
            .collect();
        let n_uncovered = count(&uncovered);
        if n_uncovered == 0 {
            if chosen.len() < self.best.len() {
                self.best = chosen.clone();
            }
            return;
        }
        // Every further vertex must strictly improve on the best solution.
        if chosen.len() + 1 >= self.best.len() {
            return;
        }
        let budget = self.best.len() - 1 - chosen.len();

        // Candidate dominators of each uncovered vertex u: N[u] \ excluded.
        // Branch on the uncovered vertex with the fewest candidates.
        let mut branch_vertex = usize::MAX;
        let mut fewest = usize::MAX;
        let mut by_cands = Vec::with_capacity(n_uncovered);
        for u in iter_bits(&uncovered) {
            let cands = count_and_not(&self.ball[u], &self.ball[u], excluded);
            if cands == 0 {
                return; // u can never be dominated
            }
            if cands < fewest {
                fewest = cands;
                branch_vertex = u;
            }
            by_cands.push((cands, u));
        }

        // Lower bound 1: even the `budget` vertices with the largest gains
        // must together cover all uncovered vertices.
        let mut gains: Vec<usize> = (0..self.n)
            .filter(|&v| !get(excluded, v))
            .map(|v| count_and(&self.ball[v], &uncovered))
            .filter(|&g| g > 0)
            .collect();
        gains.sort_unstable_by(|a, b| b.cmp(a));
        if gains.iter().take(budget).sum::<usize>() < n_uncovered {
            return;
        }
        // Lower bound 2: uncovered vertices with pairwise disjoint candidate
        // sets each need their own dominator.
        by_cands.sort_unstable();
        if self.packing_bound(by_cands.iter().map(|&(_, u)| u), excluded) > budget {
            return;
        }

        // Try each candidate for `branch_vertex`, best coverage first. After a
        // candidate has been tried, it is excluded in the remaining branches.
        let mut cands: Vec<usize> = iter_bits(&self.ball[branch_vertex])
            .filter(|&v| !get(excluded, v))
            .collect();
        cands.sort_by_key(|&v| std::cmp::Reverse(count_and(&self.ball[v], &uncovered)));
        let mut excluded = excluded.clone();
        for v in cands {
            let new_covered: Bits = covered.iter().zip(&self.ball[v]).map(|(c, b)| c | b).collect();
            chosen.push(v);
            self.search(chosen, &new_covered, &excluded);
            chosen.pop();
            set(&mut excluded, v);
            if chosen.len() + 1 >= self.best.len() {
                return;
            }
        }
    }

    fn packing_bound(&self, uncovered: impl Iterator<Item = usize>, excluded: &Bits) -> usize {
        let mut used = vec![0u64; excluded.len()];
        let mut bound = 0;
        for u in uncovered {
            let b = &self.ball[u];
            let disjoint = b.iter().zip(excluded).zip(&used).all(|((b, e), used)| b & !e & used == 0);
            if disjoint {
                bound += 1;
                for ((used, b), e) in used.iter_mut().zip(b).zip(excluded) {
                    *used |= b & !e;
                }
            }
        }
        bound
    }
}

/// Convenience wrapper: minimum dominating set of a graph with `n` vertices.
pub fn minimum_dominating_set(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    Solver::new(n, edges).solve()
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
    }

    #[test]
    fn random_graphs_match_brute_force() {
        // Small deterministic PRNG so the test needs no dependencies.
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut rand = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..300 {
            let n = (rand() % 15) as usize + 1;
            let p = rand() % 100;
            let mut edges = Vec::new();
            for a in 0..n {
                for b in a + 1..n {
                    if rand() % 100 < p {
                        edges.push((a, b));
                    }
                }
            }
            let s = minimum_dominating_set(n, &edges);
            assert!(is_dominating(n, &edges, &s));
            assert_eq!(s.len(), brute_force(n, &edges), "n={} edges={:?}", n, edges);
        }
    }
}
