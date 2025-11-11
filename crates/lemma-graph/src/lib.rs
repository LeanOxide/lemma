//! Dependency graph utilities
//!
//! This crate provides data structures and algorithms for managing
//! directed acyclic graphs (DAGs) representing build dependencies.
//!
//! # Features
//!
//! - Add nodes and edges to build a dependency graph
//! - Detect cycles in the graph
//! - Topological sorting for build order determination
//! - Transitive dependency computation
//!
//! # Example
//!
//! ```rust
//! use lemma_graph::DependencyGraph;
//!
//! let mut graph = DependencyGraph::new();
//! graph.add_node("A");
//! graph.add_node("B");
//! graph.add_edge("B", "A"); // B depends on A
//!
//! let order = graph.topological_sort().unwrap();
//! assert_eq!(order, vec!["A", "B"]);
//! ```

pub mod error;
pub mod graph;

pub use error::{Error, Result};
pub use graph::DependencyGraph;
