use anyhow::Result;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Dependency graph for topological sorting
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    nodes: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: HashMap::new(),
        }
    }

    /// Add a package node to the graph
    pub fn add_node(&mut self, pkg: &str) -> NodeIndex {
        if let Some(&idx) = self.nodes.get(pkg) {
            return idx;
        }

        let idx = self.graph.add_node(pkg.to_string());
        self.nodes.insert(pkg.to_string(), idx);
        idx
    }

    /// Add dependency edge: `from` depends on `to`
    pub fn add_edge(&mut self, from: &str, to: &str) {
        let from_idx = self.add_node(from);
        let to_idx = self.add_node(to);
        self.graph.add_edge(from_idx, to_idx, ());
    }

    /// Get topological order (dependencies first)
    pub fn topological_order(&self) -> Result<Vec<String>> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order
                .into_iter()
                .map(|idx| self.graph[idx].clone())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()),
            Err(cycle) => {
                let cycle_node = &self.graph[cycle.node_id()];
                anyhow::bail!("Circular dependency detected involving: {}", cycle_node);
            }
        }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dag() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b"); // a depends on b
        graph.add_edge("b", "c"); // b depends on c

        let order = graph.topological_order().unwrap();
        assert_eq!(order, vec!["c", "b", "a"]);
    }

    #[test]
    fn test_circular_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("c", "a"); // Creates cycle

        assert!(graph.topological_order().is_err());
    }
}
