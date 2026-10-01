# enki-apsu

Vulkan GPU memory allocator and device buffer abstractions for the Enki heterogeneous compute runtime.
Part of the [Enki](https://github.com/enkiruntime/enki) ecosystem.

## Features
- VMA Integration: Automatic Unified Memory (UMA) vs Discrete VRAM heap detection
- Buffer Device Address (BDA): Direct 64-bit GPU pointer generation for bindless architectures
- Timeline Reclamation: Deferred GPU resource cleanup synchronized via timeline semaphores
- Staging Ring Buffer: High-throughput host-to-device transfers with fallback dynamic staging
