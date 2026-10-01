use anyhow::{Context, Result};
use std::sync::{Arc, Mutex, OnceLock};

use anu::context::{EngineConfig, EnkiEngine, EnkiEngineBuilder};
use utu::GpuWindow;

use super::ambient::ActiveFlowGuard;
use super::errors::handle_execution_error;
use super::flow::Flow;

static ACTIVE_ENKI: OnceLock<Arc<Enki>> = OnceLock::new();

/// Sets the global active Enki context instance.
pub fn set_active_enki(enki: Arc<Enki>) {
    let _ = ACTIVE_ENKI.set(enki);
}

/// Retrieves a clone of the globally active Enki context.
pub fn active_enki() -> Arc<Enki> {
    ACTIVE_ENKI
        .get()
        .cloned()
        .expect("[Enki] No active Enki context found. Did you call 'Enki::init()'?")
}

/// Retrieves a clone of the underlying Vulkan compute engine (`EnkiEngine`).
pub fn active_engine() -> Arc<EnkiEngine> {
    active_enki().engine.clone()
}

/// Builder for configuring engine settings before runtime initialization.
pub struct EnkiBuilder {
    config: EngineConfig,
}

impl EnkiBuilder {
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
        }
    }

    /// Sets the application name reported to the Vulkan driver.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.config.app_name = name.into();
        self
    }

    /// Sets the maximum capacity in bytes allocated for uniform parameter uploads (ParamArena).
    pub fn param_arena_size(mut self, size_bytes: u64) -> Self {
        self.config.max_param_arena_size = Some(size_bytes);
        self
    }

    // /// Configures the maximum number of bindless sampled image descriptors.
    // pub fn max_sampled_images(mut self, count: u32) -> Self {
    //     self.config.max_sampled_images = count;
    //     self
    // }

    // /// Configures the maximum number of bindless storage image descriptors.
    // pub fn max_storage_images(mut self, count: u32) -> Self {
    //     self.config.max_storage_images = count;
    //     self
    // }

    // /// Configures the maximum number of bindless sampler descriptors.
    // pub fn max_samplers(mut self, count: u32) -> Self {
    //     self.config.max_samplers = count;
    //     self
    // }

    /// Finalizes configuration and initializes a headless GPU context.
    #[track_caller]
    pub fn init(self) -> Arc<Enki> {
        let caller = std::panic::Location::caller();
        let mut config = self.config;
        config.headless = true;
        config.caller_location = Some((caller.file(), caller.line(), caller.column()));

        match Enki::new_headless_with_config(config) {
            Ok(enki) => enki,
            Err(e) => handle_execution_error(&e),
        }
    }

    /// Finalizes configuration and initializes a windowed presentation GPU context.
    #[track_caller]
    pub fn init_windowed<W>(self, window: Arc<W>, width: u32, height: u32) -> Arc<Enki>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + Send
            + Sync
            + 'static,
    {
        let caller = std::panic::Location::caller();
        let mut config = self.config;
        config.headless = false;
        config.caller_location = Some((caller.file(), caller.line(), caller.column()));

        match Enki::new_windowed_with_config(config, window, width, height) {
            Ok(enki) => enki,
            Err(e) => handle_execution_error(&e),
        }
    }

    /// Sets the maximum number of hardware timestamp profiling queries per flow.
    pub fn max_timestamp_queries(mut self, count: u32) -> Self {
        self.config.max_timestamp_queries = count;
        self
    }
}

impl Default for EnkiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The central handle to the Enki heterogeneous GPU runtime.
///
/// Manages Vulkan instance lifecycle, hardware devices, memory allocation heaps,
/// and execution flows across host CPU and GPU silicon.
pub struct Enki {
    pub window_context: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub gpu_window: Option<Mutex<GpuWindow>>,
    pub engine: Arc<EnkiEngine>,
}

impl Enki {
    /// Returns a new `EnkiBuilder` to configure runtime options before initialization.
    pub fn builder() -> EnkiBuilder {
        EnkiBuilder::new()
    }

    /// Initializes a headless GPU compute context with default settings.
    pub fn init() -> Arc<Self> {
        Self::builder().init()
    }

    /// Initializes a windowed GPU context with swapchain presentation support.
    pub fn init_windowed<W>(window: Arc<W>, width: u32, height: u32) -> Arc<Self>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + Send
            + Sync
            + 'static,
    {
        Self::builder().init_windowed(window, width, height)
    }

    fn new_headless_with_config(config: EngineConfig) -> Result<Arc<Self>> {
        let engine = EnkiEngineBuilder::new(config)
            .build()
            .context("[Enki Core] Failed to build headless EnkiEngine")?;

        let enki = Arc::new(Self {
            window_context: None,
            gpu_window: None,
            engine: Arc::from(engine),
        });

        set_active_enki(enki.clone());
        Ok(enki)
    }

    fn new_windowed_with_config<W>(
        mut config: EngineConfig,
        window: Arc<W>,
        width: u32,
        height: u32,
    ) -> Result<Arc<Self>>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + Send
            + Sync
            + 'static,
    {
        let display_handle = window
            .display_handle()
            .map_err(|_| {
                let diag = anu::diagnostics::hw::headless_display_mismatch();
                anyhow::anyhow!("{}", anu::diagnostics::emit_diagnostic(&diag))
            })?
            .as_raw();

        let surface_extensions = ash_window::enumerate_required_extensions(display_handle)
            .map_err(|_| {
                let diag = anu::diagnostics::hw::headless_display_mismatch();
                anyhow::anyhow!("{}", anu::diagnostics::emit_diagnostic(&diag))
            })?;

        for &ext_ptr in surface_extensions {
            let cstr = unsafe { std::ffi::CStr::from_ptr(ext_ptr) };
            config.required_instance_extensions.push(cstr.to_owned());
        }

        config
            .required_device_extensions
            .push(std::ffi::CString::from(ash::khr::swapchain::NAME));

        let engine = EnkiEngineBuilder::new(config)
            .build()
            .context("[Enki Core] Failed to build windowed EnkiEngine")?;

        let engine = Arc::new(engine);

        let gpu_window = GpuWindow::new(
            &engine.instance,
            engine.raw_physical_device(),
            &engine.device,
            window.as_ref(),
            window.as_ref(),
            width,
            height,
            3,
        )
        .context("[Enki Core] Failed to create GpuWindow context")?;

        let enki = Arc::new(Self {
            window_context: Some(window as Arc<dyn std::any::Any + Send + Sync>),
            gpu_window: Some(Mutex::new(gpu_window)),
            engine,
        });

        set_active_enki(enki.clone());
        Ok(enki)
    }

    /// Returns an `Arc` clone of the currently active global Enki runtime instance.
    pub fn active() -> Arc<Self> {
        active_enki()
    }

    pub fn begin_flow(&self) -> Flow<'_> {
        Flow::new(self)
    }

    /// Records and executes an atomic compute and presentation flow, halting on execution error.
    #[track_caller]
    #[inline(always)]
    pub fn flow<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut Flow<'_>) -> R,
    {
        match self.try_flow(f) {
            Ok(output) => output,
            Err(e) => handle_execution_error(&e),
        }
    }

    /// Records and executes an atomic flow, returning an explicit `Result` on failure.
    #[track_caller]
    pub fn try_flow<R, F>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Flow<'_>) -> R,
    {
        let mut flow = self.begin_flow();
        let output = {
            let _guard = ActiveFlowGuard::enter(&mut flow);
            f(&mut flow)
        };

        if let Some(err) = flow.sticky_error.take() {
            return Err(err);
        }

        flow.try_end_flow()?;
        Ok(output)
    }

    /// Resizes the underlying presentation window swapchain dimensions.
    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        if let Some(window_mutex) = &self.gpu_window {
            let mut window = window_mutex.lock().unwrap();
            window
                .recreate(self.engine.raw_physical_device(), width, height)
                .context("[Enki Core] Swapchain recreation failed")?;
        }
        Ok(())
    }

    /// Blocks the host CPU until all in-flight GPU timeline tasks are completely idle.
    pub fn wait_idle(&self) -> Result<()> {
        self.engine.wait_idle()
    }
}
