use super::graph::{DependencyGraph, DependencyEdge};
use std::collections::HashSet;

pub struct TransitiveReductionSolver;

impl TransitiveReductionSolver {
    pub fn reduce(graph: DependencyGraph) -> DependencyGraph {
        let tasks_count = graph.tasks_count;
        let original_edges = graph.edges;

        let mut adj = vec![Vec::new(); tasks_count];
        for (edge_idx, edge) in original_edges.iter().enumerate() {
            adj[edge.source_task].push(edge_idx);
        }

        let mut redundant_edge_indices = HashSet::new();

        for u in 0..tasks_count {
            for &edge_idx in &adj[u] {
                let edge = &original_edges[edge_idx];
                let v = edge.destination_task;

                if dfs_reachability(u, v, &adj, &original_edges, edge_idx, &redundant_edge_indices, tasks_count) {
                    redundant_edge_indices.insert(edge_idx);
                }
            }
        }

        let mut reduced_edges = Vec::with_capacity(original_edges.len() - redundant_edge_indices.len());
        for (idx, edge) in original_edges.into_iter().enumerate() {
            if !redundant_edge_indices.contains(&idx) {
                reduced_edges.push(edge);
            }
        }

        DependencyGraph {
            tasks_count,
            edges: reduced_edges,
        }
    }
}

fn dfs_reachability(
    start: usize,
    target: usize,
    adj: &[Vec<usize>],
    edges: &[DependencyEdge],
    ignored_edge_idx: usize,
    redundant_indices: &HashSet<usize>,
    tasks_count: usize,
) -> bool {
    let mut visited = vec![false; tasks_count];
    let mut stack = Vec::with_capacity(tasks_count);

    stack.push(start);
    visited[start] = true;

    while let Some(curr) = stack.pop() {
        if curr == target {
            return true;
        }

        for &edge_idx in &adj[curr] {
            if edge_idx == ignored_edge_idx {
                continue;
            }

            if redundant_indices.contains(&edge_idx) {
                continue;
            }

            let edge = &edges[edge_idx];
            let next = edge.destination_task;

            if !visited[next] {
                visited[next] = true;
                stack.push(next);
            }
        }
    }

    false
}