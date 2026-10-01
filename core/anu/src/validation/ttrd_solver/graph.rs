use crate::nam_args_api::AccessIntent;
use crate::recording::queue::TaskQueue;
use crate::recording::task::RawTask;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardType {
    RAW,
    WAR,
    WAW,
}

#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub source_task: usize,
    pub destination_task: usize,
    pub resource_slot: u32,
    pub hazard_type: HazardType,
}

pub struct DependencyGraph {
    pub tasks_count: usize,
    pub edges: Vec<DependencyEdge>,
}

impl DependencyGraph {
    pub fn build<'a>(queue: &TaskQueue<'a>) -> Self {
        let tasks_count = queue.tasks.len();
        let mut edges = Vec::new();

        struct ResourceAccessHistory {
            last_writer: Option<usize>,
            last_readers: Vec<usize>,
        }

        let mut history_map: HashMap<u32, ResourceAccessHistory> = HashMap::new();

        for (curr_idx, task) in queue.tasks.iter().enumerate() {
            match task {
                RawTask::Compute(t) => {
                    for bound_res in &t.args_ctx.bound_resources {
                        let slot = bound_res.slot_index;
                        let is_write = matches!(
                            bound_res.intent,
                            AccessIntent::Write | AccessIntent::ReadWrite
                        );

                        let history =
                            history_map
                                .entry(slot)
                                .or_insert_with(|| ResourceAccessHistory {
                                    last_writer: None,
                                    last_readers: Vec::new(),
                                });

                        if is_write {
                            for &reader_idx in &history.last_readers {
                                if reader_idx != curr_idx {
                                    edges.push(DependencyEdge {
                                        source_task: reader_idx,
                                        destination_task: curr_idx,
                                        resource_slot: slot,
                                        hazard_type: HazardType::WAR,
                                    });
                                }
                            }

                            if let Some(writer_idx) = history.last_writer
                                && writer_idx != curr_idx
                            {
                                edges.push(DependencyEdge {
                                    source_task: writer_idx,
                                    destination_task: curr_idx,
                                    resource_slot: slot,
                                    hazard_type: HazardType::WAW,
                                });
                            }
                            history.last_writer = Some(curr_idx);
                            history.last_readers.clear();
                        } else {
                            if let Some(writer_idx) = history.last_writer
                                && writer_idx != curr_idx
                            {
                                edges.push(DependencyEdge {
                                    source_task: writer_idx,
                                    destination_task: curr_idx,
                                    resource_slot: slot,
                                    hazard_type: HazardType::RAW,
                                });
                            }

                            history.last_readers.push(curr_idx);
                        }
                    }
                }
                RawTask::PresentBuffer(p) => {
                    let slot = p.buffer_id;
                    let history =
                        history_map
                            .entry(slot)
                            .or_insert_with(|| ResourceAccessHistory {
                                last_writer: None,
                                last_readers: Vec::new(),
                            });

                    if let Some(writer_idx) = history.last_writer
                        && writer_idx != curr_idx
                    {
                        edges.push(DependencyEdge {
                            source_task: writer_idx,
                            destination_task: curr_idx,
                            resource_slot: slot,
                            hazard_type: HazardType::RAW,
                        });
                    }

                    history.last_readers.push(curr_idx);
                }
                RawTask::WriteTimestamp { .. } => {}
            }
        }

        Self { tasks_count, edges }
    }
}
