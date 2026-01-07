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

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn has_node(&self, path: &PathBuf) -> bool {
        self.nodes.contains(path)
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
        for (from, targets) in &self.edges {
            out_degree.insert(from.clone(), targets.len());
        }

        // PageRank flows along edges from dependents to dependencies.
        // A link from A to B (A depends on B) is a "vote" for B.

        // Build adjacency list (dependents per dependency) for score distribution.
        let mut incoming: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for (from, targets) in &self.edges {
            for to in targets {
                incoming.entry(to.clone()).or_default().push(from.clone());
            }
        }

        for _ in 0..iterations {
            let mut new_scores = HashMap::new();

            // Calculate total sink rank (nodes with no outgoing edges)
            let mut sink_rank = 0.0;
            for node in &self.nodes {
                if !self.edges.contains_key(node)
                    || self.edges.get(node).is_some_and(|e| e.is_empty())
                {
                    sink_rank += scores.get(node).unwrap_or(&0.0);
                }
            }

            // Redistribute rank from sink nodes (no outgoing edges) equally.
            let sink_contribution = sink_rank / num_nodes as f64;

            for node in &self.nodes {
                let mut incoming_score_sum = 0.0;

                if let Some(voters) = incoming.get(node) {
                    for voter in voters {
                        let voter_score = scores.get(voter).unwrap_or(&0.0);
                        let voter_out_degree = out_degree.get(voter).unwrap_or(&1);
                        incoming_score_sum += *voter_score / *voter_out_degree as f64;
                    }
                }

                let new_score = (1.0 - damping_factor) / num_nodes as f64
                    + damping_factor * (incoming_score_sum + sink_contribution);
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

        // Dependency order: A depends on B implies B precedes A.
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
                // Handle cycles or isolated nodes.
                break;
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
