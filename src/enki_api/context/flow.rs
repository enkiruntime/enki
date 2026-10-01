use anyhow::{Context, Result};
use ash::vk;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::enki_api::context::contract::{lookup_signature_contract, resolve_nam_name};
use crate::enki_api::context::errors::emit_and_abort;
use crate::enki_api::context::instance::Enki;
use crate::enki_api::resources::{GpuVec, Slice};
use crate::enki_api::space::Space;

use super::instant::GpuInstant;
use anu::diagnostics::source::Span;
use anu::nam_args_api::NamDispatchMap;
use anu::pipeline_synthesis::ComputeSynthesisInput;
use anu::recording::queue::TaskQueue;
use anu::recording::recipe::CompiledExecutionRecipe;
use anu::recording::task::{ComputeTask, PresentBufferTask, RawTask};
use anu::validation::{BorrowEngine, FrameBorrowLedger};
use utu::GpuWindow;

/// An active, coherent GPU compute and presentation execution stream.
///
/// A `Flow` batches nam (kernel) dispatches and display presentation tasks into an atomic
/// command buffer sequence synchronized by timeline semaphores.
pub struct Flow<'a> {
    pub enki: &'a Enki,
    pub queue: TaskQueue<'a>,
    pub cmd: vk::CommandBuffer,
    pub slot_idx: usize,
    pub timeline_value: u64,
    pub submitted: AtomicBool,
    pub sticky_error: Option<anyhow::Error>,
    pub borrow_ledger: FrameBorrowLedger,
    pub timestamp_count: u32,
}

impl<'a> Flow<'a> {
    /// Initializes a new Flow, acquiring a command buffer from the ring and resetting query pools.
    pub fn new(enki: &'a Enki) -> Self {
        let engine = &enki.engine;

        let current_gpu_value = engine.timeline_semaphore.get_timeline_value().unwrap_or(0);
        engine.allocator.reclaim_resources(current_gpu_value);

        let timeline_value = engine.timeline_counter.fetch_add(1, Ordering::SeqCst) + 1;
        engine
            .allocator
            .current_timeline_value
            .store(timeline_value, Ordering::Release);

        let (cmd, slot_idx) = {
            let mut ring = engine.command_ring.lock().unwrap();
            ring.acquire_next_cmd(engine.raw_device(), engine.timeline_semaphore.handle)
                .context("[Flow] Failed to acquire command buffer from ring")
                .unwrap()
        };

        let max_queries = engine.max_timestamp_queries;
        let start_query = (slot_idx as u32) * max_queries;

        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            engine
                .raw_device()
                .begin_command_buffer(cmd, &begin_info)
                .unwrap();

            engine.raw_device().cmd_reset_query_pool(
                cmd,
                engine.query_pool,
                start_query,
                max_queries,
            );
        }

        Self {
            enki,
            queue: TaskQueue::new(),
            cmd,
            slot_idx,
            timeline_value,
            submitted: AtomicBool::new(false),
            sticky_error: None,
            borrow_ledger: FrameBorrowLedger::new(),
            timestamp_count: 0,
        }
    }

    /// Queues an owned GPU vector for direct presentation to the display window.
    ///
    /// # Temporal Presentation Hazard
    /// Once queued for presentation, any subsequent dispatch (`.run()` or `.run_unchecked()`) within the same flow that
    /// attempts to mutate (`&mut`) this buffer halts execution with diagnostic **`error[E1010]`**.
    /// Read-only access (`&`) after presentation remains permitted.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic **`error[E2002]`** if element count is less than `width * height`.
    #[track_caller]
    pub fn present<T: Copy + Send + Sync + 'static>(&mut self, pixels: &GpuVec<T>) {
        let caller = std::panic::Location::caller();
        self.validate_and_present_raw(
            pixels.slot_index,
            pixels._inner.buffer(),
            pixels.offset as u64,
            pixels.len(),
            pixels.stride(),
            caller,
        );
    }

    /// Queues a contiguous GPU slice for direct presentation to the display window.
    ///
    /// # Temporal Presentation Hazard
    /// Once queued for presentation, any subsequent dispatch (`.run()` or `.run_unchecked()`) within the same flow that
    /// attempts to mutate (`&mut`) this buffer halts execution with diagnostic **`error[E1010]`**.
    /// Read-only access (`&`) after presentation remains permitted.
    ///
    /// # Diagnostics
    /// Halts execution with diagnostic **`error[E2002]`** if element count is less than `width * height`.    #[track_caller]
    #[track_caller]
    pub fn present_slice<T: Copy + Send + Sync + 'static>(&mut self, pixels: &Slice<T>) {
        let caller = std::panic::Location::caller();
        self.validate_and_present_raw(
            pixels.slot_index,
            pixels._inner.buffer(),
            pixels.offset as u64,
            pixels.len(),
            pixels.stride(),
            caller,
        );
    }
    fn validate_and_present_raw(
        &mut self,
        buffer_id: u32,
        buffer: ash::vk::Buffer,
        offset: u64,
        element_count: usize,
        _element_stride: usize,
        caller: &'static std::panic::Location<'static>,
    ) {
        self.borrow_ledger
            .mark_queued_for_presentation(buffer_id as usize);

        let (width, height) = if let Some(window_mutex) = &self.enki.gpu_window {
            let window = window_mutex.lock().unwrap();
            (window.extent.width, window.extent.height)
        } else {
            (0, 0)
        };

        if width == 0 || height == 0 {
            return;
        }

        let required_elements = (width * height) as usize;

        if element_count < required_elements {
            let diag = anu::diagnostics::rt::present_dimension_mismatch(
                width,
                height,
                element_count,
                caller,
            );
            emit_and_abort(&diag);
        }

        let task = PresentBufferTask {
            buffer_id,
            buffer,
            offset,
            width,
            height,
        };

        self.push_task(RawTask::PresentBuffer(task));
    }

    pub(crate) fn nam_impl_direct<F>(
        &mut self,
        space: &Space,
        args_ctx: anu::nam_args_api::IngressContext<'static>,
        mut map: NamDispatchMap,
    ) -> Result<()>
    where
        F: 'static,
    {
        let engine = &self.enki.engine;
        let nam_name = resolve_nam_name::<F>()?;

        map.nam_name = nam_name.clone();

        if let Some(contract) = lookup_signature_contract(&nam_name) {
            map.expected_contract = Some(contract);
        }

        let required_param_bytes = args_ctx.pack().len() as u64;
        let arena_capacity = engine.param_arena.size_bytes();

        if required_param_bytes > arena_capacity {
            let diag =
                anu::diagnostics::hw::param_arena_overflow(required_param_bytes, arena_capacity);
            crate::enki_api::context::errors::emit_and_abort(&diag);
        }

        if let Err(violation) = BorrowEngine::validate_dispatch(&map, &mut self.borrow_ledger) {
            let diag = anu::diagnostics::ContractDiagnosticBuilder::from_violation(violation, &map);
            emit_and_abort(&diag);
        }

        let dispatch = space.resolve_dispatch(&engine.hardware_profile);

        let input = ComputeSynthesisInput {
            nam_name: nam_name.clone(),
            local_size: dispatch.local_size,
            host_manifest_dir: std::env::var("CARGO_MANIFEST_DIR").ok(),
            caller_file_path: map.call_site.map(|(f, _, _)| f.to_string()),
            arg_descriptors: args_ctx.descriptors.clone(),
        };

        let artifact = engine
            .synthesizer
            .synthesize_compute(engine, &input)
            .context("[Flow] JIT synthesis failed for nam")?;

        let total_threads = (dispatch.global_size.0 as u64)
            * (dispatch.global_size.1 as u64)
            * (dispatch.global_size.2 as u64);
        let required_stack_bytes = (artifact.stack_size_per_thread as u64) * total_threads;

        let (stack_bda, stack_buffer) = if required_stack_bytes > 0 {
            match apsu::GpuStackBuffer::allocate(engine.allocator.clone(), required_stack_bytes) {
                Ok(Some(buf)) => {
                    let bda = buf.device_address();
                    (bda, Some(buf))
                }
                Ok(None) => (0, None),
                Err(alloc_err) => {
                    let mut diag = anu::diagnostics::hw::stack_overflow(
                        &alloc_err,
                        dispatch.global_size,
                        artifact.stack_size_per_thread,
                        None,
                    );

                    if let Some((file, line, col)) = map.call_site {
                        diag.add_span(Span::primary(file, line as usize, col as usize, 1));
                    }

                    emit_and_abort(&diag);
                }
            }
        } else {
            (0, None)
        };

        let task = ComputeTask {
            pipeline: artifact.pipeline,
            layout: artifact.layout,
            grid_size: dispatch.global_size,
            local_size: dispatch.local_size,
            args_ctx,
            stack_bda,
            _stack_buffer: stack_buffer,
        };

        self.push_task(RawTask::Compute(task));
        Ok(())
    }

    /// Finalizes and submits the recorded flow to the GPU queue, halting on failure.
    pub fn end_flow(self) {
        if let Err(e) = self.try_end_flow() {
            crate::enki_api::context::errors::handle_execution_error(&e);
        }
    }

    /// Finalizes and submits the recorded flow to the GPU queue, returning a `Result`.
    pub fn try_end_flow(self) -> Result<()> {
        if self.submitted.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let engine = &self.enki.engine;
        let recipe = engine.compile_recipe(&self.queue);

        let has_present = self
            .queue
            .tasks
            .iter()
            .any(|t| matches!(t, RawTask::PresentBuffer(_)));

        if has_present && let Some(window_mutex) = &self.enki.gpu_window {
            self.submit_windowed(&recipe, window_mutex)?;
        } else {
            self.submit_headless(&recipe)?;
        }

        {
            let mut ring = engine.command_ring.lock().unwrap();
            ring.update_slot_timeline(self.slot_idx, self.timeline_value);
        }

        Ok(())
    }

    fn submit_headless(&self, recipe: &CompiledExecutionRecipe) -> Result<()> {
        let engine = &self.enki.engine;
        let device = engine.raw_device();

        recipe
            .replay(
                engine,
                self.cmd,
                &self.queue,
                self.slot_idx,
                engine.query_pool,
            )
            .context("[Flow] Recipe replay failed in headless submit")?;

        unsafe {
            device.end_command_buffer(self.cmd)?;
        }

        let cmd_buffers = [self.cmd];
        let signal_semaphores = [engine.timeline_semaphore.handle];
        let signal_values = [self.timeline_value];

        let mut timeline_info =
            vk::TimelineSemaphoreSubmitInfo::default().signal_semaphore_values(&signal_values);

        let submit_info = vk::SubmitInfo::default()
            .push_next(&mut timeline_info)
            .command_buffers(&cmd_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            device.queue_submit(engine.queue.handle, &[submit_info], vk::Fence::null())?;
            engine
                .timeline_semaphore
                .wait_timeline(self.timeline_value, std::time::Duration::from_secs(5))?;
        }

        Ok(())
    }

    fn submit_windowed(
        &self,
        recipe: &CompiledExecutionRecipe,
        window_mutex: &Mutex<GpuWindow>,
    ) -> Result<()> {
        let engine = &self.enki.engine;
        let device = engine.raw_device();

        let mut window_lock = window_mutex.lock().unwrap();

        let (image_index, _) = window_lock
            .acquire_next_image(std::time::Duration::from_secs(5))
            .context("[Flow] Failed to acquire next swapchain image")?;

        recipe
            .replay(
                engine,
                self.cmd,
                &self.queue,
                self.slot_idx,
                engine.query_pool,
            )
            .context("[Flow] Recipe replay failed in windowed submit")?;

        for task in &self.queue.tasks {
            if let RawTask::PresentBuffer(p) = task {
                window_lock.cmd_copy_buffer_to_image(
                    self.cmd,
                    image_index,
                    p.buffer,
                    p.offset,
                    p.width,
                    p.height,
                );
            }
        }

        unsafe {
            device.end_command_buffer(self.cmd)?;
        }

        let wait_semaphores = [window_lock.current_image_acquired_semaphore()];
        let wait_stages =
            [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::TRANSFER];
        let signal_semaphores = [
            engine.timeline_semaphore.handle,
            window_lock.current_render_finished_semaphore(),
        ];
        let signal_values = [self.timeline_value, 0];

        let mut timeline_info =
            vk::TimelineSemaphoreSubmitInfo::default().signal_semaphore_values(&signal_values);

        let cmd_buffers = [self.cmd];
        let submit_info = vk::SubmitInfo::default()
            .push_next(&mut timeline_info)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&cmd_buffers)
            .signal_semaphores(&signal_semaphores);

        let in_flight_fence = window_lock.current_in_flight_fence();

        unsafe {
            device.queue_submit(engine.queue.handle, &[submit_info], in_flight_fence)?;
        }

        window_lock
            .present_image(engine.queue.handle, image_index)
            .context("[Flow] Failed to present swapchain image")?;

        Ok(())
    }

    /// Records a hardware timestamp query mark on the GPU timeline for profiling.
    #[track_caller]
    pub fn mark(&mut self) -> GpuInstant {
        let caller = std::panic::Location::caller();
        let engine = &self.enki.engine;
        let max_queries = engine.max_timestamp_queries;

        if self.timestamp_count >= max_queries {
            let diag = anu::diagnostics::hw::timestamp_queries_exceeded(
                self.timestamp_count + 1,
                max_queries,
                Some(caller),
            );
            emit_and_abort(&diag);
        }

        let global_query_slot = (self.slot_idx as u32) * max_queries + self.timestamp_count;
        self.timestamp_count += 1;

        self.write_timestamp(global_query_slot, vk::PipelineStageFlags2::ALL_COMMANDS);

        GpuInstant {
            query_slot: global_query_slot,
            timeline_value: self.timeline_value,
            timestamp_period: engine.timestamp_period,
        }
    }

    /// Records a timestamp query into the active flow.
    pub fn write_timestamp(&mut self, query_index: u32, stage: vk::PipelineStageFlags2) {
        self.push_task(RawTask::WriteTimestamp { query_index, stage });
    }

    /// Pushes a low-level task onto the internal execution queue.
    pub fn push_task(&mut self, task: RawTask<'a>) {
        self.queue.push(task);
    }
}

impl<'a> Drop for Flow<'a> {
    fn drop(&mut self) {
        if !self.submitted.load(Ordering::SeqCst) {
            let _ = self.enki.engine.wait_idle();
        }
    }
}
