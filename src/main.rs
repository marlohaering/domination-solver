use petgraph::graph::NodeIndex;
use petgraph::Graph;
use petgraph::Undirected;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;

#[derive(Debug, Clone)]
struct SolutionSet<'a> {
    g: &'a Graph<&'a str, (), Undirected>,
    s: HashSet<NodeIndex>,
    t: HashSet<NodeIndex>,
    w: HashMap<NodeIndex, usize>,
    max_w_value_node: Option<NodeIndex>,
}

impl SolutionSet<'_> {
    fn new<'a>(
        g: &'a Graph<&'a str, (), Undirected>,
        s: HashSet<NodeIndex>,
        t: HashSet<NodeIndex>,
    ) -> SolutionSet<'a> {
        let mut ss = SolutionSet {
            g,
            s,
            t,
            w: HashMap::new(),
            max_w_value_node: None,
        };
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
        let mut current_max_w_value: usize = 0;
        for node in self.g.node_indices() {
            let w_value = self.get_w_value(node);
            self.w.insert(node, w_value);
            if w_value > current_max_w_value {
                self.max_w_value_node = Some(node);
                current_max_w_value = w_value;
            }
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

    fn is_dominated(&self) -> bool {
        self.get_uncovered_nodes().len() == 0
    }

    fn print_infos(&self) {
        println!("S = {:?}", self.s);
        println!("T = {:?}", self.t);
        for (node, w_value) in &self.w {
            let weight = self.g.raw_nodes()[node.index()].weight;
            println!("{} = {}", weight, w_value);
        }
        println!("max_w_value = {:?}", self.get_max_w_value());
        println!("lower_bound = {:?}", self.get_lower_bound());
        if let Some(max_node) = self.max_w_value_node {
            println!(
                "max_w_value_node = {:?}",
                self.g.raw_nodes()[max_node.index()].weight
            );
        }
    }

    fn create_new_solutions_sets(&self, a: NodeIndex) -> (SolutionSet, SolutionSet) {
        let mut new_s = self.s.clone();
        new_s.insert(a);
        let ss_with = SolutionSet::new(self.g, new_s, self.t.clone());

        let mut new_t = self.t.clone();
        new_t.insert(a);
        let ss_without = SolutionSet::new(self.g, self.s.clone(), new_t);
        (ss_with, ss_without)
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

    let first_ss = SolutionSet::new(
        &graph,
        HashSet::new(), //vec![node_index(1)].iter().cloned().collect(),
        HashSet::new(),
    );

    let mut found_domination = false;
    let mut lower_bounds: (f32, SolutionSet) = (first_ss.get_lower_bound(), first_ss);

    // first_ss.print_infos();

    // while !found_domination {

    let max_w_value_node = lower_bounds.1.max_w_value_node;
    let new_solution_sets = lower_bounds
        .1
        .create_new_solutions_sets(max_w_value_node.expect("No w value node found!"))
        .clone();

    if new_solution_sets.0.get_lower_bound() < lower_bounds.0 {
        lower_bounds.0 = new_solution_sets.0.get_lower_bound();
        lower_bounds.1 = new_solution_sets.0;
    }

    if new_solution_sets.1.get_lower_bound() < lower_bounds.0 {
        lower_bounds.0 = new_solution_sets.1.get_lower_bound();
        lower_bounds.1 = new_solution_sets.1;
    }
    // }
}
