//! Impact analysis for code changes
//!
//! Analyzes the impact of code changes by tracing dependencies and calculating risk scores.
//! Perfect for code review and PR workflows.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::utils::graph::DependencyGraph;

/// Impact analysis result for changed files
#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    /// Files that were directly changed
    pub changed_files: Vec<PathBuf>,
    /// Files directly affected (depend on changed files)
    pub directly_affected: Vec<PathBuf>,
    /// Files transitively affected (depend on affected files)
    pub transitively_affected: Vec<PathBuf>,
    /// Risk score for the change (0.0 - 1.0)
    pub risk_score: f64,
    /// Risk breakdown
    pub risk_factors: Vec<RiskFactor>,
    /// Suggested tests to run
    pub suggested_tests: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RiskFactor {
    pub name: String,
    pub description: String,
    pub weight: f64,
}

impl std::fmt::Display for ImpactAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Impact Analysis")?;
        writeln!(f, "===============")?;
        writeln!(f)?;

        writeln!(f, "Changed files: {}", self.changed_files.len())?;
        for file in &self.changed_files {
            writeln!(f, "  * {}", file.display())?;
        }

        writeln!(f)?;
        writeln!(f, "Directly affected: {}", self.directly_affected.len())?;
        for file in self.directly_affected.iter().take(10) {
            writeln!(f, "  -> {}", file.display())?;
        }
        if self.directly_affected.len() > 10 {
            writeln!(f, "  ... and {} more", self.directly_affected.len() - 10)?;
        }

        writeln!(f)?;
        writeln!(
            f,
            "Transitively affected: {}",
            self.transitively_affected.len()
        )?;

        writeln!(f)?;
        writeln!(
            f,
            "Risk score: {:.0}% ({})",
            self.risk_score * 100.0,
            risk_level(self.risk_score)
        )?;

        if !self.risk_factors.is_empty() {
            writeln!(f)?;
            writeln!(f, "Risk factors:")?;
            for factor in &self.risk_factors {
                writeln!(f, "  - {}: {}", factor.name, factor.description)?;
            }
        }

        if !self.suggested_tests.is_empty() {
            writeln!(f)?;
            writeln!(f, "Suggested tests:")?;
            for test in self.suggested_tests.iter().take(5) {
                writeln!(f, "  cargo test {}", test.display())?;
            }
        }

        Ok(())
    }
}

fn risk_level(score: f64) -> &'static str {
    if score >= 0.8 {
        "CRITICAL"
    } else if score >= 0.6 {
        "HIGH"
    } else if score >= 0.4 {
        "MEDIUM"
    } else if score >= 0.2 {
        "LOW"
    } else {
        "MINIMAL"
    }
}

/// Impact analyzer for code changes
pub struct ImpactAnalyzer<'a> {
    #[allow(dead_code)]
    graph: &'a DependencyGraph,
    /// Reverse dependency map (who depends on me?)
    reverse_deps: HashMap<PathBuf, HashSet<PathBuf>>,
    /// PageRank scores for centrality
    pagerank: HashMap<PathBuf, f64>,
}

impl<'a> ImpactAnalyzer<'a> {
    /// Create a new impact analyzer
    pub fn new(graph: &'a DependencyGraph) -> Self {
        // Build reverse dependency map
        let mut reverse_deps: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

        for (from, targets) in graph.get_edges() {
            for to in targets {
                reverse_deps
                    .entry(to.clone())
                    .or_default()
                    .insert(from.clone());
            }
        }

        let pagerank = graph.calculate_pagerank();

        Self {
            graph,
            reverse_deps,
            pagerank,
        }
    }

    /// Analyze impact of changed files
    pub fn analyze(&self, changed_files: &[PathBuf], all_files: &[PathBuf]) -> ImpactAnalysis {
        let changed_set: HashSet<_> = changed_files.iter().cloned().collect();

        // Find directly affected files (files that depend on changed files)
        let mut directly_affected = HashSet::new();
        for changed in &changed_set {
            if let Some(dependents) = self.reverse_deps.get(changed) {
                for dep in dependents {
                    if !changed_set.contains(dep) {
                        directly_affected.insert(dep.clone());
                    }
                }
            }
        }

        // Find transitively affected files (BFS through reverse deps)
        let mut transitively_affected = HashSet::new();
        let mut queue: VecDeque<_> = directly_affected.iter().cloned().collect();
        let mut visited: HashSet<_> = changed_set.iter().cloned().collect();
        visited.extend(directly_affected.iter().cloned());

        while let Some(file) = queue.pop_front() {
            if let Some(dependents) = self.reverse_deps.get(&file) {
                for dep in dependents {
                    if !visited.contains(dep) {
                        visited.insert(dep.clone());
                        transitively_affected.insert(dep.clone());
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // Calculate risk score
        let (risk_score, risk_factors) =
            self.calculate_risk(&changed_set, &directly_affected, &transitively_affected);

        // Find suggested tests
        let suggested_tests = self.find_relevant_tests(
            &changed_set,
            &directly_affected,
            &transitively_affected,
            all_files,
        );

        ImpactAnalysis {
            changed_files: changed_files.to_vec(),
            directly_affected: directly_affected.into_iter().collect(),
            transitively_affected: transitively_affected.into_iter().collect(),
            risk_score,
            risk_factors,
            suggested_tests,
        }
    }

    /// Calculate risk score based on multiple factors
    fn calculate_risk(
        &self,
        changed: &HashSet<PathBuf>,
        direct: &HashSet<PathBuf>,
        transitive: &HashSet<PathBuf>,
    ) -> (f64, Vec<RiskFactor>) {
        let mut score = 0.0;
        let mut factors = Vec::new();

        // Factor 1: Number of affected files (blast radius)
        let total_affected = direct.len() + transitive.len();
        let blast_radius = (total_affected as f64 / 100.0).min(1.0);
        score += blast_radius * 0.3;
        if total_affected > 10 {
            factors.push(RiskFactor {
                name: "Blast radius".into(),
                description: format!("{} files affected", total_affected),
                weight: blast_radius,
            });
        }

        // Factor 2: Central files changed (high PageRank)
        let max_pagerank: f64 = changed
            .iter()
            .filter_map(|p| self.pagerank.get(p))
            .copied()
            .fold(0.0, f64::max);
        let centrality_risk = (max_pagerank * 10.0).min(1.0);
        score += centrality_risk * 0.3;
        if centrality_risk > 0.3 {
            factors.push(RiskFactor {
                name: "Centrality".into(),
                description: "Core/central files modified".into(),
                weight: centrality_risk,
            });
        }

        // Factor 3: Critical file patterns
        let critical_patterns = [
            "config",
            "auth",
            "security",
            "payment",
            "database",
            "migration",
        ];
        let has_critical = changed.iter().any(|p| {
            let name = p.to_string_lossy().to_lowercase();
            critical_patterns.iter().any(|pat| name.contains(pat))
        });
        if has_critical {
            score += 0.2;
            factors.push(RiskFactor {
                name: "Critical files".into(),
                description: "Security/config/payment files changed".into(),
                weight: 0.2,
            });
        }

        // Factor 4: Number of files changed (complexity)
        let change_complexity = (changed.len() as f64 / 20.0).min(1.0);
        score += change_complexity * 0.2;
        if changed.len() > 5 {
            factors.push(RiskFactor {
                name: "Change size".into(),
                description: format!("{} files changed", changed.len()),
                weight: change_complexity,
            });
        }

        (score.min(1.0), factors)
    }

    /// Find test files that are relevant to the changes
    fn find_relevant_tests(
        &self,
        changed: &HashSet<PathBuf>,
        direct: &HashSet<PathBuf>,
        transitive: &HashSet<PathBuf>,
        all_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        let affected: HashSet<_> = changed
            .iter()
            .chain(direct.iter())
            .chain(transitive.iter())
            .collect();

        // Find test files that import affected modules
        let test_files: Vec<_> = all_files
            .iter()
            .filter(|f| {
                let name = f.to_string_lossy().to_lowercase();
                name.contains("test") || name.contains("spec") || name.ends_with("_test.rs")
            })
            .cloned()
            .collect();

        // Match tests to affected files by name similarity
        let mut relevant_tests = Vec::new();
        for test in test_files {
            let test_name = test
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
                .replace("_test", "")
                .replace("test_", "");

            for affected_file in &affected {
                let affected_name = affected_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if affected_name.contains(&test_name) || test_name.contains(&affected_name) {
                    relevant_tests.push(test.clone());
                    break;
                }
            }
        }

        relevant_tests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_deps() {
        let mut graph = DependencyGraph::new();
        let a = PathBuf::from("a.rs");
        let b = PathBuf::from("b.rs");
        let c = PathBuf::from("c.rs");

        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_node(c.clone());
        graph.add_edge(b.clone(), a.clone()); // b depends on a
        graph.add_edge(c.clone(), b.clone()); // c depends on b

        let analyzer = ImpactAnalyzer::new(&graph);

        // Check reverse deps: a should have b as dependent
        assert!(analyzer.reverse_deps.get(&a).unwrap().contains(&b));
        // b should have c as dependent
        assert!(analyzer.reverse_deps.get(&b).unwrap().contains(&c));
    }

    #[test]
    fn test_impact_analysis() {
        let mut graph = DependencyGraph::new();
        let core = PathBuf::from("core.rs");
        let utils = PathBuf::from("utils.rs");
        let handler = PathBuf::from("handler.rs");

        graph.add_node(core.clone());
        graph.add_node(utils.clone());
        graph.add_node(handler.clone());
        graph.add_edge(utils.clone(), core.clone()); // utils depends on core
        graph.add_edge(handler.clone(), utils.clone()); // handler depends on utils

        let analyzer = ImpactAnalyzer::new(&graph);
        let all_files = vec![core.clone(), utils.clone(), handler.clone()];

        // Change core.rs
        let analysis = analyzer.analyze(&[core.clone()], &all_files);

        // utils should be directly affected
        assert!(analysis.directly_affected.contains(&utils));
        // handler should be transitively affected
        assert!(analysis.transitively_affected.contains(&handler));
    }

    #[test]
    fn test_risk_level() {
        assert_eq!(risk_level(0.9), "CRITICAL");
        assert_eq!(risk_level(0.7), "HIGH");
        assert_eq!(risk_level(0.5), "MEDIUM");
        assert_eq!(risk_level(0.3), "LOW");
        assert_eq!(risk_level(0.1), "MINIMAL");
    }
}
