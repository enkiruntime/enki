use super::task::RawTask;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct TaskQueue<'a> {
    pub tasks: Vec<RawTask<'a>>,
}

impl<'a> TaskQueue<'a> {
    pub fn new() -> Self {
        Self {
            tasks: Vec::with_capacity(16),
        }
    }

    pub fn push(&mut self, task: RawTask<'a>) {
        self.tasks.push(task);
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn calculate_structure_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();

        for task in &self.tasks {
            match task {
                RawTask::Compute(t) => {
                    0u8.hash(&mut hasher);
                    t.pipeline.hash(&mut hasher);
                    t.grid_size.0.hash(&mut hasher);
                    t.grid_size.1.hash(&mut hasher);
                    t.grid_size.2.hash(&mut hasher);
                    t.local_size.0.hash(&mut hasher);
                    t.local_size.1.hash(&mut hasher);
                    t.local_size.2.hash(&mut hasher);

                    for desc in &t.args_ctx.descriptors {
                        desc.hash(&mut hasher);
                    }
                }
                RawTask::PresentBuffer(p) => {
                    1u8.hash(&mut hasher);
                    p.buffer_id.hash(&mut hasher);
                    p.width.hash(&mut hasher);
                    p.height.hash(&mut hasher);
                }
                RawTask::WriteTimestamp { query_index, stage } => {
                    4u8.hash(&mut hasher);
                    query_index.hash(&mut hasher);
                    stage.as_raw().hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }
}

impl<'a> Default for TaskQueue<'a> {
    fn default() -> Self {
        Self::new()
    }
}
