# The Mental Model

To write effective GPU algorithms with Enki, developers do not need to learn a foreign shader language. However, it is essential to understand **how standard Rust types map onto physical GPU hardware**.

This section deconstructs the conceptual and architectural bridges between host Rust and graphics silicon:

- **[Rust References to 64-bit BDA](references_to_bda.md):** How Enki completely eliminates Vulkan descriptor sets by routing 64-bit Buffer Device Addresses (BDA) through push constants.
- **[The SPMD Per-Cell Principle](spmd_principle.md):** The conceptual distinction between 1:1 scalar cell access (`&mut T`) and unconstrained global slices (`&mut [T]`).
- **[Spatial Execution & Topology](space_topology.md):** Navigating multi-dimensional problem spaces (1D, 2D, 3D), workgroup tiles, and coordinate helper functions using `Space`.
