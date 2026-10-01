pub mod barrier_generator;
pub mod graph;
pub mod reduction;

pub use barrier_generator::{BarrierGenerator, TaskBarriers};
pub use graph::{DependencyEdge, DependencyGraph, HazardType};
pub use reduction::TransitiveReductionSolver;

use crate::recording::queue::TaskQueue;

pub fn solve_optimal_barriers<'a>(queue: &TaskQueue<'a>) -> Vec<TaskBarriers> {
    let initial_graph = DependencyGraph::build(queue);

    let reduced_graph = TransitiveReductionSolver::reduce(initial_graph);

    BarrierGenerator::generate(&reduced_graph, queue)
}
