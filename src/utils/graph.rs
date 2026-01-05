use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    nodes: HashSet<PathBuf>,
    edges: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, path: PathBuf) {
        self.nodes.insert(path);
    }

    pub fn add_edge(&mut self, from: PathBuf, to: PathBuf) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.edges.entry(from).or_default().insert(to);
    }

    pub fn get_nodes(&self) -> &HashSet<PathBuf> {
        &self.nodes
    }

    pub fn get_edges(&self) -> &HashMap<PathBuf, HashSet<PathBuf>> {
        &self.edges
    }

    /// Calculates PageRank for all nodes in the graph.
    /// Returns a map of PathBuf -> relative importance score (0.0 to 1.0ish).
    pub fn calculate_pagerank(&self) -> HashMap<PathBuf, f64> {
        let damping_factor = 0.85;
        let iterations = 20;
        let num_nodes = self.nodes.len();

        if num_nodes == 0 {
            return HashMap::new();
        }

        let initial_score = 1.0 / num_nodes as f64;
        let mut scores: HashMap<PathBuf, f64> = self
            .nodes
            .iter()
            .map(|n| (n.clone(), initial_score))
            .collect();

        // Pre-calculate outgoing edges count for each node
        let mut out_degree: HashMap<PathBuf, usize> = HashMap::new();
        // Since our edges map is `edges: HashMap<PathBuf, HashSet<PathBuf>>`
        // `edges[from]` gives targets. `from` depends on `to`.
        // Wait, standard PageRank: A link from A to B is a "vote" for B.
        // In dependency graph: A imports B. A is analyzing B. A "votes" for B?
        // Usually, if many files import utils.rs, utils.rs is important.
        // So A -> B (dependency) means B gets a vote from A.
        // Correct. A depends on B. Edge is A -> B.
        // PageRank flows along edges. A passes score to B.

        for (from, targets) in &self.edges {
            out_degree.insert(from.clone(), targets.len());
        }

        // We also need to know which nodes map to which targets inversely for efficient iteration?
        // PageRank formula: PR(u) = (1-d)/N + d * Sum(PR(v) / L(v)) for all v linking to u.
        // In our graph, v links to u if v imports u.
        // That is exactly our `edges` map: key depends on value.
        // So `edges` stores outgoing links. `edges.get(v)` returns list of `u`s.
        // But to calculate PR(u), we need incoming links (who links TO u).
        // Let's build the reverse map: node -> list of voters (dependents)

        let mut incoming: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for (from, targets) in &self.edges {
            for to in targets {
                incoming.entry(to.clone()).or_default().push(from.clone());
            }
        }

        for _ in 0..iterations {
            let mut new_scores = HashMap::new();

            for node in &self.nodes {
                let mut incoming_score_sum = 0.0;

                if let Some(voters) = incoming.get(node) {
                    for voter in voters {
                        let voter_score = scores.get(voter).unwrap_or(&0.0);
                        let voter_out_degree = out_degree.get(voter).unwrap_or(&1); // Should match targets.len()
                        incoming_score_sum += *voter_score / *voter_out_degree as f64;
                    }
                }

                // Add sink logic? If a node has no outgoing edges, it's a sink.
                // Standard PageRank distributes sink rank to all nodes.
                // For simplified version, we ignore sinks or assume self-loop?
                // Let's stick to simple version first.

                let new_score =
                    (1.0 - damping_factor) / num_nodes as f64 + damping_factor * incoming_score_sum;
                new_scores.insert(node.clone(), new_score);
            }
            scores = new_scores;
        }

        scores
    }

    /// Sorts files topologically.
    /// `comparator`: A function to compare two independent items (tie-breaker).
    /// If A depends on B, B comes before A.
    pub fn sort_topologically<F>(&self, mut comparator: F) -> Vec<PathBuf>
    where
        F: FnMut(&PathBuf, &PathBuf) -> std::cmp::Ordering,
    {
        use topological_sort::TopologicalSort;
        let mut ts = TopologicalSort::<PathBuf>::new();

        // Seed with all nodes
        for node in &self.nodes {
            ts.insert(node.clone());
        }

        // Add dependencies: A -> B means A depends on B.
        // We want B before A.
        // ts.add_dependency(dependency, dependent)
        // dependent depends on dependency.
        // So B is dependency, A is dependent.
        // add_dependency(B, A).

        // self.edges: key (from) -> value (targets/tos)
        // from depends on to.
        // A (from) -> B (to)
        // A depends on B.
        // So B comes before A.
        // add_dependency(B, A) => add_dependency(to, from)

        for (from, targets) in &self.edges {
            for to in targets {
                ts.add_dependency(to.clone(), from.clone());
            }
        }

        let mut result = Vec::new();
        while !ts.is_empty() {
            // Pop all items with no dependencies
            let mut batch = ts.pop_all();
            if batch.is_empty() {
                // Cycle detected? topological-sort crate handles this by returning empty pop if cycle exists but not empty.
                // Force break to avoid infinite loop.
                if !ts.is_empty() {
                    // Cycle fallback: dump remaining arbitrary
                    // Or maybe we can just extend?
                    // The crate documentation says `pop_all` returns empty if cycle.
                    // We could iterate `len()`?
                    // For now, let's just break.
                    break;
                }
            }

            // Sort this batch using the importance score
            batch.sort_by(|a, b| comparator(a, b));

            result.extend(batch);
        }

        // If items remaining (cycles), append them purely by score
        if result.len() < self.nodes.len() {
            // Find missing
            let included: HashSet<&PathBuf> = result.iter().collect();
            let mut remaining: Vec<PathBuf> = self
                .nodes
                .iter()
                .filter(|n| !included.contains(n))
                .cloned()
                .collect();
            remaining.sort_by(|a, b| comparator(a, b));
            result.extend(remaining);
        }

        result
    }
}
