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
        let nodes: HashSet<NodeIndex> = self
            .g
            .node_indices()
            .collect::<HashSet<NodeIndex>>()
            .difference(&self.s)
            .cloned()
            .collect::<HashSet<NodeIndex>>()
            .difference(&self.t)
            .cloned()
            .collect();
        for node in nodes {
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
        let s_nodes: HashSet<&str> = self
            .s
            .iter()
            .map(|&node| self.g.raw_nodes()[node.index()].weight)
            .collect();
        println!("S = {:?}", s_nodes);

        let t_nodes: HashSet<&str> = self
            .t
            .iter()
            .map(|&node| self.g.raw_nodes()[node.index()].weight)
            .collect();
        println!("T = {:?}", t_nodes);
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
}

fn create_new_solutions_sets<'b>(
    ss: &SolutionSet<'b>,
    a: NodeIndex,
) -> (SolutionSet<'b>, SolutionSet<'b>) {
    let sss = ss.clone();
    let mut new_s = sss.s.clone();
    new_s.insert(a);
    let ss_with = SolutionSet::new(sss.g, new_s, sss.t.clone());

    let mut new_t = sss.t.clone();
    new_t.insert(a);
    let ss_without = SolutionSet::new(sss.g, sss.s.clone(), new_t);

    (ss_with, ss_without)
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

fn create_test_graph() -> Graph<&'static str, (), Undirected> {
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
    graph
}

fn create_test_graph2() -> Graph<&'static str, (), Undirected> {
    let mut graph = Graph::<&str, (), Undirected>::new_undirected();
    graph.add_node("a"); // 0
    graph.add_node("b"); // 1
    graph.add_node("c"); // 2
    graph.add_node("d"); // 2
    graph.add_node("e"); // 2
    graph.add_node("f"); // 2
    graph.add_node("g"); // 2
    graph.add_node("h"); // 2
    graph.extend_with_edges(&[(0, 1), (0, 2)]);
    graph
}

fn main() {
    let graph = create_test_graph2();

    let first_ss = SolutionSet::new(
        &graph,
        HashSet::new(), //vec![node_index(1)].iter().cloned().collect(),
        HashSet::new(),
    );

    let mut solution_sets = vec![(std::f32::INFINITY, first_ss.clone())];
    let mut lowest_ss: SolutionSet;

    loop {
        lowest_ss = solution_sets[0].1.clone();
        lowest_ss.print_infos();

        if lowest_ss.is_dominated() {
            lowest_ss.print_infos();
            break;
        }

        solution_sets.remove(0);

        let (ss_with_node, ss_without_node) = {
            let max_w_value_node = lowest_ss.max_w_value_node.expect("No max w value found!");
            let next_ssets = create_new_solutions_sets(&lowest_ss, max_w_value_node);

            let ss_with_node = next_ssets.0.clone();
            let ss_without_node = next_ssets.1.clone();

            (ss_with_node, ss_without_node)
        };

        println!("New solution sets:");

        ss_with_node.print_infos();
        ss_without_node.print_infos();

        solution_sets.push((ss_with_node.get_lower_bound(), ss_with_node.clone()));
        solution_sets.push((ss_without_node.get_lower_bound(), ss_without_node.clone()));
        solution_sets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        // if ss_with_node.get_lower_bound() < lowest_bound {
        //     lowest_bound = ss_with_node.get_lower_bound();
        //     lowest_ss = ss_with_node;
        //     // println!("Found lower bound with node");
        //     // lowest_ss.print_infos();
        // }
        // if ss_without_node.get_lower_bound() < lowest_bound {
        //     lowest_bound = ss_without_node.get_lower_bound();
        //     lowest_ss = ss_without_node;
        //     // println!("Found lower without node");
        //     // lowest_ss.print_infos();
        // }
    }
}
