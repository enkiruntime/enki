use crate::nam_args_api::IngressContext;
use ash::vk;

pub struct ComputeTask<'a> {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub grid_size: (u32, u32, u32),
    pub local_size: (u32, u32, u32),
    pub args_ctx: IngressContext<'a>,
    pub stack_bda: u64,
    pub _stack_buffer: Option<apsu::GpuStackBuffer>,
}

pub struct PresentBufferTask {
    pub buffer_id: u32,
    pub buffer: vk::Buffer,
    pub offset: u64,
    pub width: u32,
    pub height: u32,
}

pub enum RawTask<'a> {
    Compute(ComputeTask<'a>),
    PresentBuffer(PresentBufferTask),
    WriteTimestamp {
        query_index: u32,
        stage: vk::PipelineStageFlags2,
    },
}

impl<'a> Default for ComputeTask<'a> {
    fn default() -> Self {
        Self {
            pipeline: vk::Pipeline::null(),
            layout: vk::PipelineLayout::null(),
            grid_size: (1, 1, 1),
            local_size: (1, 1, 1),
            args_ctx: IngressContext::new(),
            stack_bda: 0,
            _stack_buffer: None,
        }
    }
}
