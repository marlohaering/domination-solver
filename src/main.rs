use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;

// use petgraph::dot::{Config, Dot};
use petgraph::graph::node_index;
// use petgraph::graph::Neighbors;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use petgraph::Undirected;

#[derive(Debug)]
struct SolutionSet<'a> {
    g: &'a Graph<&'a str, (), Undirected>,
    s: HashSet<NodeIndex>,
    t: HashSet<NodeIndex>,
    w: HashMap<NodeIndex, usize>,
}

impl SolutionSet<'_> {
    fn new<'a>(
        g: &'a Graph<&'a str, (), Undirected>,
        s: HashSet<NodeIndex>,
        t: HashSet<NodeIndex>,
        w: HashMap<NodeIndex, usize>,
    ) -> SolutionSet<'a> {
        let mut ss = SolutionSet { g, s, t, w };
        ss.update_w_values();
        return ss;
    }

    fn get_covered_nodes(&self) -> HashSet<NodeIndex> {
        let mut covered_nodes = HashSet::<NodeIndex>::new();
        for s_node in self.s.iter() {
            covered_nodes.insert(s_node.clone());
            for node in self.g.neighbors(*s_node) {
                covered_nodes.insert(node.clone());
            }
        }
        covered_nodes
    }

    fn get_uncovered_nodes(&self) -> HashSet<NodeIndex> {
        let uncovered_nodes: HashSet<NodeIndex> = self.g.node_indices().collect();
        uncovered_nodes
            .difference(&self.get_covered_nodes())
            .cloned()
            .collect()
    }

    fn get_w_value(&self, a: NodeIndex) -> usize {
        self.g
            .get_ball(a)
            .difference(&self.get_covered_nodes())
            .count()
    }

    fn update_w_values(&mut self) {
        for node in self.g.node_indices() {
            let w_value = self.get_w_value(node);
            self.w.insert(node, w_value);
        }
    }

    fn get_max_w_value(&self) -> usize {
        let max_w_value = self.w.values().max().expect("No w values found!");
        *max_w_value
    }

    fn get_lower_bound(&self) -> f32 {
        if self.get_max_w_value() != 0 {
            return self.s.len() as f32
                + self.get_uncovered_nodes().len() as f32 / self.get_max_w_value() as f32;
        }
        0f32
    }
}

trait BallType {
    fn get_ball(&self, a: NodeIndex) -> HashSet<NodeIndex>;
}

impl BallType for Graph<&str, (), Undirected> {
    fn get_ball(&self, a: NodeIndex) -> HashSet<NodeIndex> {
        let mut neighbors: HashSet<NodeIndex> = self.neighbors(a).clone().collect();
        neighbors.insert(a);
        neighbors
    }
}

impl BallType for SolutionSet<'_> {
    fn get_ball(&self, a: NodeIndex) -> HashSet<NodeIndex> {
        let mut neighbors: HashSet<NodeIndex> = self.g.neighbors(a).clone().collect();
        neighbors.insert(a);
        neighbors
    }
}

fn main() {
    let mut graph = Graph::<&str, (), Undirected>::new_undirected();
    graph.add_node("a"); // 0
    graph.add_node("b"); // 1
    graph.add_node("c"); // 2
    graph.add_node("d"); // 3
    graph.add_node("e"); // 4
    graph.add_node("f"); // 5
    graph.add_node("g"); // 6
    graph.add_node("h"); // 7
    graph.add_node("i"); // 8
    graph.add_node("j"); // 9
    graph.extend_with_edges(&[
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (0, 5),
        (2, 5),
        (2, 9),
        (3, 4),
        (3, 6),
        (5, 8),
        (7, 8),
        (1, 7),
    ]);
    let graph = graph;

    let test_ss = SolutionSet::new(
        &graph,
        vec![node_index(1)].iter().cloned().collect(),
        HashSet::new(),
        HashMap::new(),
    );

    println!("test_ss.w = {:?}", test_ss.w);
    println!("max_w_value = {:?}", test_ss.get_max_w_value());
    let lower_bound = test_ss.get_lower_bound();
    println!("lower_bound = {:?}", lower_bound);
}
