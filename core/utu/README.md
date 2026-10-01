# enki-utu

Vulkan window surface integration, swapchain management, and display presentation for the Enki heterogeneous compute runtime.
Part of the [Enki](https://github.com/enkiruntime/enki) ecosystem.

## Features
- Window Surface Integration: Cross-platform window surface initialization via `raw-window-handle` and `ash-window`
- Dynamic Swapchain Recreation: Seamless resize handling and format selection
- Synchronization2 Pipeline Barriers: Direct compute-to-present layout transitions with zero tearing
- Multi-Frame In-Flight Synchronization: Triple-buffering support with fences and acquire/render semaphores
