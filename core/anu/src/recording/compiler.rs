use crate::pipeline_synthesis::cache::RAMTaskCache;
use crate::recording::queue::TaskQueue;
use crate::recording::recipe::{BakeCommand, CompiledExecutionRecipe};
use crate::recording::task::RawTask;
use ash::vk;

pub struct FrameCompiler;

impl FrameCompiler {
    pub fn compile<'a>(
        queue: &TaskQueue<'a>,
        _descriptor_set: vk::DescriptorSet,
        cache: &RAMTaskCache,
    ) -> CompiledExecutionRecipe {
        let mut commands = Vec::with_capacity(queue.tasks.len() * 2);
        let mut barrier_batches = Vec::new();

        let optimal_barriers = crate::validation::ttrd_solver::solve_optimal_barriers(queue);

        let mut task_offsets = vec![0; queue.tasks.len()];
        let mut current_offset = 0;

        for (idx, task) in queue.tasks.iter().enumerate() {
            match task {
                RawTask::Compute(t) => {
                    if cache.get_by_pipeline(t.pipeline).is_some() {
                        let barriers = &optimal_barriers[idx];
                        if !barriers.memory_barriers.is_empty() {
                            let batch_idx = barrier_batches.len();
                            barrier_batches.push(barriers.clone());
                            commands.push(BakeCommand::PipelineBarrier {
                                batch_idx,
                                task_idx: idx,
                            });
                        }

                        commands.push(BakeCommand::BindPipeline {
                            bind_point: vk::PipelineBindPoint::COMPUTE,
                            pipeline: t.pipeline,
                        });

                        let packed_args = t.args_ctx.pack();
                        let aligned_args_size = (packed_args.len() + 255) & !255;

                        task_offsets[idx] = current_offset;
                        current_offset += aligned_args_size;

                        commands.push(BakeCommand::PushConstants {
                            stages: vk::ShaderStageFlags::COMPUTE,
                            layout: t.layout,
                            task_idx: idx,
                        });

                        commands.push(BakeCommand::Dispatch { task_idx: idx });
                    }
                }

                RawTask::PresentBuffer(_) => {
                    let barriers = &optimal_barriers[idx];
                    if !barriers.memory_barriers.is_empty() {
                        let batch_idx = barrier_batches.len();
                        barrier_batches.push(barriers.clone());
                        commands.push(BakeCommand::PipelineBarrier {
                            batch_idx,
                            task_idx: idx,
                        });
                    }
                }

                RawTask::WriteTimestamp { query_index, stage } => {
                    commands.push(BakeCommand::WriteTimestamp {
                        query_index: *query_index,
                        stage: *stage,
                    });
                }
            }
        }

        CompiledExecutionRecipe {
            commands,
            barrier_batches,
            task_offsets,
            total_frame_param_size: current_offset,
        }
    }
}
