use crate::context::EnkiEngine;
use crate::recording::queue::TaskQueue;
use crate::recording::task::RawTask;
use crate::validation::TaskBarriers;
use ash::vk;

#[derive(Clone, Debug)]
pub enum BakeCommand {
    BindPipeline {
        bind_point: vk::PipelineBindPoint,
        pipeline: vk::Pipeline,
    },
    PushConstants {
        stages: vk::ShaderStageFlags,
        layout: vk::PipelineLayout,
        task_idx: usize,
    },
    PipelineBarrier {
        batch_idx: usize,
        task_idx: usize,
    },
    Dispatch {
        task_idx: usize,
    },
    WriteTimestamp {
        query_index: u32,
        stage: vk::PipelineStageFlags2,
    },
}

pub struct CompiledExecutionRecipe {
    pub commands: Vec<BakeCommand>,
    pub barrier_batches: Vec<TaskBarriers>,
    pub task_offsets: Vec<usize>,
    pub total_frame_param_size: usize,
}

impl CompiledExecutionRecipe {
    pub fn replay<'a>(
        &self,
        engine: &EnkiEngine,
        cmd: vk::CommandBuffer,
        current_queue: &TaskQueue<'a>,
        slot_idx: usize,
        query_pool: vk::QueryPool,
    ) -> anyhow::Result<()> {
        let device = engine.raw_device();

        for command in &self.commands {
            match command {
                BakeCommand::BindPipeline {
                    bind_point,
                    pipeline,
                } => unsafe {
                    device.cmd_bind_pipeline(cmd, *bind_point, *pipeline);
                },

                BakeCommand::PushConstants {
                    stages,
                    layout,
                    task_idx,
                } => {
                    let (packed_args, grid_size, stack_bda) = match &current_queue.tasks[*task_idx]
                    {
                        RawTask::Compute(t) => (t.args_ctx.pack(), t.grid_size, t.stack_bda),
                        _ => (Vec::new(), (1, 1, 1), 0),
                    };

                    let static_offset = self.task_offsets[*task_idx];
                    let target_offset = (slot_idx * self.total_frame_param_size) + static_offset;
                    let param_arena_ptr = engine.param_arena.base_address() + target_offset as u64;

                    if !packed_args.is_empty() {
                        engine.param_arena.write_raw(
                            target_offset,
                            &packed_args,
                            Some(&engine.transfer_manager),
                        )?;
                    }

                    let mut push_bytes = [0u8; 32];
                    push_bytes[0..8].copy_from_slice(&param_arena_ptr.to_ne_bytes());
                    push_bytes[8..12].copy_from_slice(&grid_size.0.to_ne_bytes());
                    push_bytes[12..16].copy_from_slice(&grid_size.1.to_ne_bytes());
                    push_bytes[16..20].copy_from_slice(&grid_size.2.to_ne_bytes());
                    push_bytes[24..32].copy_from_slice(&stack_bda.to_ne_bytes());

                    unsafe {
                        device.cmd_push_constants(cmd, *layout, *stages, 0, &push_bytes);
                    }
                }

                BakeCommand::PipelineBarrier { batch_idx, .. } => {
                    let batch = &self.barrier_batches[*batch_idx];
                    let dependency_info =
                        vk::DependencyInfo::default().memory_barriers(&batch.memory_barriers);

                    unsafe {
                        device.cmd_pipeline_barrier2(cmd, &dependency_info);
                    }
                }

                BakeCommand::Dispatch { task_idx } => {
                    if let RawTask::Compute(task) = &current_queue.tasks[*task_idx] {
                        let group_count_x = task.grid_size.0.div_ceil(task.local_size.0);
                        let group_count_y = task.grid_size.1.div_ceil(task.local_size.1);
                        let group_count_z = task.grid_size.2.div_ceil(task.local_size.2);

                        unsafe {
                            device.cmd_dispatch(cmd, group_count_x, group_count_y, group_count_z);
                        }
                    }
                }

                BakeCommand::WriteTimestamp { query_index, stage } => unsafe {
                    device.cmd_write_timestamp2(cmd, *stage, query_pool, *query_index);
                },
            }
        }
        Ok(())
    }
}
