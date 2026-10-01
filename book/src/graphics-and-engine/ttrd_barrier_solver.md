# The Automatic Barrier Solver (TTRD)

In raw Vulkan programming, managing synchronization is notoriously difficult. Developers must manually insert execution and memory barriers (`vkCmdPipelineBarrier2`), explicitly specifying pipeline stages and access masks to prevent data hazards.

Manual synchronization presents two major risks:
1. **Under-synchronization:** Missing a barrier causes silent data corruption, screen tearing, and non-deterministic race conditions.
2. **Over-synchronization:** Inserting redundant or overly broad barriers serializes the GPU execution units, creating pipeline bubbles that destroy performance.

Enki eliminates manual synchronization entirely. Through its **Transitive Reduction Dependency Solver (TTRD)**, the runtime derives the mathematically optimal set of pipeline barriers automatically.

> **Note:** If you are not interested in the underlying runtime implementation, you can safely ignore this section. This is an optional architectural overview.

---

## 1. Leveraging Rust Mutability Semantics

How does the runtime know what memory barriers are required without developer annotations?

Enki leverages the semantic guarantees already encoded in Rust's type system:
- When a `#[nam]` declares `&T` or `&[T]`, the argument is tagged internally as `AccessIntent::Read`.
- When a `#[nam]` declares `&mut T` or `&mut [T]`, the argument is tagged as `AccessIntent::Write` (or `ReadWrite`).

Because each physical buffer allocation in VRAM possesses a unique internal engine slot ID, the runtime tracks the chronological memory access history for every resource across all tasks queued within an `enki.flow`.

---

## 2. Classical Data Hazards

As tasks are pushed to the active queue, the engine builds a directed dependency graph by detecting classical hazard conditions on each memory slot:

- **RAW (Read-After-Write):** Task `A` writes to a buffer; Task `B` subsequently reads from it. Task `B` must wait for Task `A`'s writes to be made visible.
- **WAR (Write-After-Read):** Task `A` reads from a buffer; Task `B` subsequently writes to it. Task `B` must not overwrite memory before Task `A` completes reading.
- **WAW (Write-After-Write):** Task `A` writes to a buffer; Task `B` subsequently overwrites it. Write orders must be strictly preserved.

---

## 3. The Transitive Reduction Algorithm

In a multi-pass compute workflow, naive hazard tracking creates numerous redundant dependencies.

For example, consider three sequential tasks operating on the same buffer:
- Task `A` writes data (Write).
- Task `B` reads and updates data (Read / Write).
- Task `C` reads the final data (Read).

A naive dependency system inserts three barriers: `A` `B`, `B` `C`, and `A` `C`.

However, because Task `B` already depends on Task `A`, and Task `C` depends on Task `B`, the direct dependency from `A` `C` is **mathematically redundant**. Inserting a barrier for `A` `C` forces the GPU to stall unnecessarily.

### Graph Reduction via Reachability
Enki applies graph-theoretic **Transitive Reduction** using depth-first search (DFS) reachability analysis:

> If a directed path already exists between task `u` and task `v` through an intermediate task `w` (`u` to `w` to `v`), the direct edge `u` to `v` is pruned from the dependency graph.

After reduction, the remaining edges represent the **minimum necessary and sufficient set of hardware barriers**.

---
