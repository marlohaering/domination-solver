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
    w: HashMap<NodeIndex, i32>,
}

impl SolutionSet<'_> {
    fn new<'a>(
        g: &'a Graph<&'a str, (), Undirected>,
        s: HashSet<NodeIndex>,
        t: HashSet<NodeIndex>,
        w: HashMap<NodeIndex, i32>,
    ) -> SolutionSet<'a> {
        let ss = SolutionSet { g, s, t, w };
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
}

trait BallType {
    fn get_ball(&self, a: NodeIndex) -> HashSet<NodeIndex>;
}

impl BallType for Graph<&str, (), Undirected> {
    fn get_ball(&self, a: NodeIndex) -> HashSet<NodeIndex> {
        self.neighbors(a).clone().collect()
    }
}

impl BallType for SolutionSet<'_> {
    fn get_ball(&self, a: NodeIndex) -> HashSet<NodeIndex> {
        self.g.neighbors(a).clone().collect()
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

    // let nodes = graph.get_ball(node_index(2));
    // nodes.for_each(|n| println!("{:?}", n));

    // println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));

    let test_ss = SolutionSet::new(
        &graph,
        vec![node_index(0)].iter().cloned().collect(),
        HashSet::new(),
        HashMap::new(),
    );

    // print!("{:?}", test_ss);
    println!("{:?}", test_ss.get_covered_nodes());
    println!("{:?}", test_ss.get_uncovered_nodes());
    println!("{:?}", test_ss.get_w_value(node_index(5)));

    // let neighbors = graph.neighbors(node_index(0));
    // neighbors.clone().for_each(|x| println!("{:?}", x));
}
