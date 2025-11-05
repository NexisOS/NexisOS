use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use anyhow::Result;

pub struct DependencyResolver {
    graph: DiGraph<String, ()>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
        }
    }
    
    pub fn add_package(&mut self, package: &Package) -> NodeIndex {
        self.graph.add_node(package.name.clone())
    }
    
    pub fn add_dependency(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, ());
    }
    
    pub fn resolve(&self) -> Result<Vec<String>> {
        let sorted = toposort(&self.graph, None)
            .map_err(|_| anyhow!("Circular dependency detected"))?;
        
        Ok(sorted
            .into_iter()
            .map(|idx| self.graph[idx].clone())
            .collect())
    }
}
