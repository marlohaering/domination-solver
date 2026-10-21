//! Command line front end.
//!
//! Usage: `domination-solver [-j THREADS] [FILE]`
//!
//! The graph is read from FILE (or stdin), one entry per line: `a b` adds an
//! edge, a single name adds an isolated vertex. `#` starts a comment.
//! Without input from a file or pipe, a built-in test graph is solved.
//! By default all CPU cores are used.

use std::collections::HashMap;
use std::io::{IsTerminal, Read};
use std::time::Instant;

const TEST_GRAPH: &str = "\
a b\na c\na d\na e\na f\nc f\nc j\nd e\nd g\nf i\nh i\nb h\n";

fn parse(input: &str) -> (Vec<String>, Vec<(usize, usize)>) {
    let mut names = Vec::new();
    let mut index = HashMap::new();
    let mut edges = Vec::new();
    for line in input.lines() {
        let line = line.split('#').next().unwrap();
        let ids: Vec<usize> = line
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .map(|name| {
                *index.entry(name.to_string()).or_insert_with(|| {
                    names.push(name.to_string());
                    names.len() - 1
                })
            })
            .collect();
        for pair in ids.windows(2) {
            edges.push((pair[0], pair[1]));
        }
    }
    (names, edges)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut path = None;
    while let Some(arg) = args.next() {
        if arg == "-j" {
            threads = args.next().and_then(|t| t.parse().ok()).expect("-j needs a number");
        } else {
            path = Some(arg);
        }
    }
    let input = match path {
        Some(path) => std::fs::read_to_string(&path).expect("cannot read input file"),
        None if !std::io::stdin().is_terminal() => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).unwrap();
            s
        }
        None => TEST_GRAPH.to_string(),
    };
    let (names, edges) = parse(&input);
    let start = Instant::now();
    let set = domination_solver::minimum_dominating_set_parallel(names.len(), &edges, threads);
    let set_names: Vec<&str> = set.iter().map(|&v| names[v].as_str()).collect();
    println!("vertices: {}, edges: {}", names.len(), edges.len());
    println!("domination number: {}", set.len());
    println!("dominating set: {}", set_names.join(" "));
    println!("time: {:?} ({} threads)", start.elapsed(), threads);
}
