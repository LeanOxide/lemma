//! Dependency graph implementation using petgraph

use crate::error::{Error, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Topo;
use petgraph::Direction;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// A directed acyclic graph (DAG) for tracking dependencies
///
/// This is a thin wrapper around petgraph's DiGraph that provides
/// a more ergonomic API for our use case and enforces DAG properties.
///
/// Nodes represent build targets (modules, libraries, etc.) and edges
/// represent dependencies (A -> B means "A depends on B").
#[derive(Debug, Clone)]
pub struct DependencyGraph<T>
where
    T: Clone + Eq + Hash + Debug,
{
    /// The underlying petgraph DiGraph
    graph: DiGraph<T, ()>,

    /// Map from node data to graph node index for efficient lookups
    node_map: HashMap<T, NodeIndex>,
}

impl<T> Default for DependencyGraph<T>
where
    T: Clone + Eq + Hash + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DependencyGraph<T>
where
    T: Clone + Eq + Hash + Debug,
{
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a node to the graph
    ///
    /// Returns Ok if the node was added, or Err if it already exists.
    pub fn add_node(&mut self, node: T) -> Result<()> {
        if self.node_map.contains_key(&node) {
            return Err(Error::DuplicateNode(format!("{:?}", node)));
        }

        let index = self.graph.add_node(node.clone());
        self.node_map.insert(node, index);
        Ok(())
    }

    /// Add a node only if it doesn't already exist
    ///
    /// Unlike `add_node`, this doesn't return an error if the node exists.
    pub fn add_node_if_missing(&mut self, node: T) {
        if !self.node_map.contains_key(&node) {
            let index = self.graph.add_node(node.clone());
            self.node_map.insert(node, index);
        }
    }

    /// Add an edge from `from` to `to`, meaning "`from` depends on `to`"
    ///
    /// Both nodes must already exist in the graph.
    pub fn add_edge(&mut self, from: T, to: T) -> Result<()> {
        let from_idx = self
            .node_map
            .get(&from)
            .ok_or_else(|| Error::NodeNotFound(format!("{:?}", from)))?;
        let to_idx = self
            .node_map
            .get(&to)
            .ok_or_else(|| Error::NodeNotFound(format!("{:?}", to)))?;

        self.graph.add_edge(*from_idx, *to_idx, ());
        Ok(())
    }

    /// Add an edge, creating nodes if they don't exist
    pub fn add_edge_with_nodes(&mut self, from: T, to: T) {
        self.add_node_if_missing(from.clone());
        self.add_node_if_missing(to.clone());

        if let (Some(&from_idx), Some(&to_idx)) = (self.node_map.get(&from), self.node_map.get(&to))
        {
            self.graph.add_edge(from_idx, to_idx, ());
        }
    }

    /// Get all nodes in the graph
    pub fn nodes(&self) -> Vec<T> {
        self.node_map.keys().cloned().collect()
    }

    /// Get the direct dependencies of a node
    pub fn dependencies(&self, node: &T) -> Option<Vec<T>> {
        let idx = self.node_map.get(node)?;
        let deps: Vec<T> = self
            .graph
            .neighbors_directed(*idx, Direction::Outgoing)
            .map(|idx| self.graph[idx].clone())
            .collect();
        Some(deps)
    }

    /// Get the nodes that depend on this node
    pub fn dependents(&self, node: &T) -> Option<Vec<T>> {
        let idx = self.node_map.get(node)?;
        let deps: Vec<T> = self
            .graph
            .neighbors_directed(*idx, Direction::Incoming)
            .map(|idx| self.graph[idx].clone())
            .collect();
        Some(deps)
    }

    /// Check if the graph contains a node
    pub fn contains(&self, node: &T) -> bool {
        self.node_map.contains_key(node)
    }

    /// Get the number of nodes in the graph
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// Perform a topological sort of the graph
    ///
    /// Returns nodes in an order such that for every edge from A to B,
    /// B appears before A in the result (dependencies before dependents).
    ///
    /// Returns an error if the graph contains a cycle.
    pub fn topological_sort(&self) -> Result<Vec<T>> {
        // Check for cycles first
        if let Err(cycle) = self.check_cycles() {
            return Err(cycle);
        }

        // Perform topological sort using petgraph's Topo iterator
        // Note: petgraph's Topo returns nodes such that for edge u->v, u comes before v
        // But our edges represent "depends on", so we need to reverse the order
        let mut topo = Topo::new(&self.graph);
        let mut result = Vec::new();

        while let Some(idx) = topo.next(&self.graph) {
            result.push(self.graph[idx].clone());
        }

        // Reverse to get dependencies before dependents
        result.reverse();
        Ok(result)
    }

    /// Check for cycles in the graph
    ///
    /// Returns Ok if the graph is acyclic, or Err with a cycle if one exists.
    fn check_cycles(&self) -> Result<()> {
        // Use petgraph's is_cyclic_directed check
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            // Find a cycle for better error reporting
            if let Some(cycle) = self.find_cycle() {
                return Err(Error::CyclicDependency(format!("{:?}", cycle)));
            }
            return Err(Error::CyclicDependency(
                "Cycle detected (unable to identify nodes)".to_string(),
            ));
        }
        Ok(())
    }

    /// Find a cycle in the graph, if one exists
    fn find_cycle(&self) -> Option<Vec<T>> {
        // Simple DFS-based cycle detection
        use petgraph::visit::{depth_first_search, Control, DfsEvent};
        use std::collections::HashSet;

        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        let mut cycle = None;

        depth_first_search(&self.graph, self.graph.node_indices(), |event| {
            match event {
                DfsEvent::Discover(n, _) => {
                    rec_stack.insert(n);
                    path.push(n);
                }
                DfsEvent::Finish(n, _) => {
                    rec_stack.remove(&n);
                    if path.last() == Some(&n) {
                        path.pop();
                    }
                }
                DfsEvent::BackEdge(_, target) => {
                    // Found a back edge - this indicates a cycle
                    if let Some(pos) = path.iter().position(|&n| n == target) {
                        cycle = Some(
                            path[pos..]
                                .iter()
                                .map(|&idx| self.graph[idx].clone())
                                .collect(),
                        );
                        return Control::Break(());
                    }
                }
                _ => {}
            }
            Control::Continue
        });

        cycle
    }

    /// Get all transitive dependencies of a node
    ///
    /// Returns all nodes that the given node depends on, directly or indirectly.
    pub fn transitive_dependencies(&self, node: &T) -> Result<Vec<T>> {
        let idx = self
            .node_map
            .get(node)
            .ok_or_else(|| Error::NodeNotFound(format!("{:?}", node)))?;

        // Use petgraph's DFS to find all reachable nodes
        use petgraph::visit::Dfs;
        let mut dfs = Dfs::new(&self.graph, *idx);
        let mut deps = Vec::new();

        while let Some(visited) = dfs.next(&self.graph) {
            if visited != *idx {
                // Don't include the node itself
                deps.push(self.graph[visited].clone());
            }
        }

        Ok(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::new();
        assert!(graph.add_node("A").is_ok());
        assert!(graph.add_node("A").is_err()); // Duplicate
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_node("A").unwrap();
        graph.add_node("B").unwrap();
        assert!(graph.add_edge("B", "A").is_ok()); // B depends on A
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = DependencyGraph::new();
        graph.add_node("A").unwrap();
        graph.add_node("B").unwrap();
        graph.add_node("C").unwrap();

        graph.add_edge("C", "B").unwrap(); // C depends on B
        graph.add_edge("B", "A").unwrap(); // B depends on A

        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_topological_sort_diamond() {
        let mut graph = DependencyGraph::new();
        graph.add_node("A").unwrap();
        graph.add_node("B").unwrap();
        graph.add_node("C").unwrap();
        graph.add_node("D").unwrap();

        // Diamond dependency: D -> {B, C} -> A
        graph.add_edge("D", "B").unwrap();
        graph.add_edge("D", "C").unwrap();
        graph.add_edge("B", "A").unwrap();
        graph.add_edge("C", "A").unwrap();

        let order = graph.topological_sort().unwrap();
        // A must come first, D must come last
        assert_eq!(order[0], "A");
        assert_eq!(order[3], "D");
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_node("A").unwrap();
        graph.add_node("B").unwrap();
        graph.add_node("C").unwrap();

        graph.add_edge("A", "B").unwrap();
        graph.add_edge("B", "C").unwrap();
        graph.add_edge("C", "A").unwrap(); // Creates a cycle

        let result = graph.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = DependencyGraph::new();
        graph.add_node("A").unwrap();
        graph.add_node("B").unwrap();
        graph.add_node("C").unwrap();
        graph.add_node("D").unwrap();

        graph.add_edge("D", "C").unwrap();
        graph.add_edge("C", "B").unwrap();
        graph.add_edge("B", "A").unwrap();

        let deps = graph.transitive_dependencies(&"D").unwrap();
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&"A"));
        assert!(deps.contains(&"B"));
        assert!(deps.contains(&"C"));
    }

    #[test]
    fn test_add_edge_with_nodes() {
        let mut graph = DependencyGraph::new();
        graph.add_edge_with_nodes("B", "A");

        assert!(graph.contains(&"A"));
        assert!(graph.contains(&"B"));
        assert_eq!(graph.len(), 2);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec!["A", "B"]);
    }

    #[test]
    fn test_dependencies_and_dependents() {
        let mut graph = DependencyGraph::new();
        graph.add_edge_with_nodes("C", "A");
        graph.add_edge_with_nodes("C", "B");

        // C depends on A and B
        let deps = graph.dependencies(&"C").unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"A"));
        assert!(deps.contains(&"B"));

        // A is depended on by C
        let dependents = graph.dependents(&"A").unwrap();
        assert_eq!(dependents.len(), 1);
        assert!(dependents.contains(&"C"));
    }
}
