# Graphics & The Barrier Engine

While Enki is a general-purpose compute platform, it natively integrates compute execution with real-time display presentation and automated hardware synchronization.

This section covers two high-level systems:

- **[Real-Time Presentation (`flow.present`)](presentation_pipeline.md):** Integrating compute pipelines directly with display windows (GLFW) and managing dynamic swapchain resizing.
- **[The Automatic Barrier Solver (TTRD)](ttrd_barrier_solver.md):** How Enki leverages Rust reference mutability to completely eliminate manual Vulkan pipeline barriers and memory hazards behind the scenes.
