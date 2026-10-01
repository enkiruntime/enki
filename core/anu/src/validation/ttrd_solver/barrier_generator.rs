use super::graph::DependencyGraph;
use crate::nam_args_api::AccessIntent;
use crate::recording::queue::TaskQueue;
use crate::recording::task::RawTask;
use ash::vk;

#[derive(Clone, Debug)]
pub struct TaskBarriers {
    pub memory_barriers: Vec<vk::MemoryBarrier2<'static>>,
}

pub struct BarrierGenerator;

impl BarrierGenerator {
    pub fn generate<'a>(graph: &DependencyGraph, queue: &TaskQueue<'a>) -> Vec<TaskBarriers> {
        let mut baked_barriers: Vec<TaskBarriers> = (0..graph.tasks_count)
            .map(|_| TaskBarriers {
                memory_barriers: Vec::new(),
            })
            .collect();

        for edge in &graph.edges {
            let u = edge.source_task;
            let v = edge.destination_task;

            let src_intent = get_resource_info(&queue.tasks[u], edge.resource_slot);
            let dst_intent = get_resource_info(&queue.tasks[v], edge.resource_slot);

            let src_stage = get_pipeline_stage(&queue.tasks[u]);
            let dst_stage = get_pipeline_stage(&queue.tasks[v]);

            let src_access = get_access_mask(src_intent, &queue.tasks[u]);
            let dst_access = get_access_mask(dst_intent, &queue.tasks[v]);

            let memory_barrier = vk::MemoryBarrier2::default()
                .src_stage_mask(src_stage)
                .src_access_mask(src_access)
                .dst_stage_mask(dst_stage)
                .dst_access_mask(dst_access);

            baked_barriers[v].memory_barriers.push(memory_barrier);
        }

        baked_barriers
    }
}

fn get_resource_info<'a>(task: &RawTask<'a>, slot_index: u32) -> AccessIntent {
    match task {
        RawTask::Compute(t) => {
            for res in &t.args_ctx.bound_resources {
                if res.slot_index == slot_index {
                    return res.intent;
                }
            }
            AccessIntent::Read
        }
        RawTask::PresentBuffer(_) => AccessIntent::Read,
        _ => AccessIntent::Read,
    }
}

fn get_pipeline_stage<'a>(task: &RawTask<'a>) -> vk::PipelineStageFlags2 {
    match task {
        RawTask::Compute(_) => vk::PipelineStageFlags2::COMPUTE_SHADER,
        RawTask::PresentBuffer(_) => vk::PipelineStageFlags2::TRANSFER,
        _ => vk::PipelineStageFlags2::ALL_COMMANDS,
    }
}

fn get_access_mask(intent: AccessIntent, task: &RawTask) -> vk::AccessFlags2 {
    let is_write = matches!(intent, AccessIntent::Write | AccessIntent::ReadWrite);

    if is_write {
        vk::AccessFlags2::SHADER_WRITE | vk::AccessFlags2::SHADER_READ
    } else {
        match task {
            RawTask::PresentBuffer(_) => vk::AccessFlags2::TRANSFER_READ,
            _ => vk::AccessFlags2::SHADER_READ,
        }
    }
}
