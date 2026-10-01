# Rejected Patterns on Silicon (E0001 - E0010)

When compiling a `#[nam]`, Enki's backend analyzes the function's intermediate representation (IR) to ensure it can execute deterministically on parallel hardware.

The following language patterns cannot be lowered to graphics silicon and will trigger compiler diagnostics:

---

## 1. Dynamic Heap Allocation (`error[E0002]`)

GPU compute units execute across thousands of concurrent execution cells without a dynamic memory heap manager (no `malloc` or `free`).

Using types that allocate heap memory dynamically (such as `Vec<T>`, `String`, or `Box<T>`) inside a `#[nam]` is rejected:

```text
error[E0002]: dynamic heap allocation is not supported inside #[nam]
  --> src/main.rs:13:17
   |
13 |     let mut v = vec![0f32; N];
   |                 ^^^^^^^^^ heap allocation occurs here
   |
   = note: GPU nams execute in parallel across silicon cells without a dynamic
           memory heap manager.
   = help: use fixed-size stack arrays `[T; N]` or pre-allocate a `GpuVec` on
           the CPU before dispatch.
```

---

## 2. Panics and Stack Unwinding (`error[E0003]`)

Graphics silicon lacks stack unwinding tables and landing pad machinery. Code running on the GPU must be non-panicking.

Calling `panic!()`, `.unwrap()`, `.expect()`, or panicking index operations inside a `#[nam]` will fail compilation:

```text
error[E0003]: explicit panic or unwinding assertion inside #[nam]
  --> src/main.rs:22:9
   |
22 |         panic!("invalid condition");
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ panic or assertion occurs here
   |
   = note: GPU silicon lacks stack unwinding and landing pad machinery; execution must be non-panicking.
   = help: avoid panicking APIs (e.g. `.unwrap()`, `assert!`) and handle bounds via conditional checks.
```

---

## 3. Dynamic Dispatch & Trait Objects (`error[E0010]`)

GPU pipelines require static control flow and register allocation known at compile time. Functions cannot be dispatched dynamically through runtime virtual method tables (`&dyn Trait`):

```text
error[E0010]: dynamic dispatch and indirect calls are not supported inside #[nam]
  --> src/main.rs:30:5
   |
30 |     trait_obj.execute();
   |     ^^^^^^^^^^^^^^^^^^^ indirect function call occurs here
   |
   = note: GPU pipelines require static control flow and cannot invoke functions through runtime pointers or vtables (`&dyn Trait`).
   = help: use concrete types, static generics (`impl Trait`), or an `enum` with pattern matching (`match`).
```

---

## 4. Unbounded Recursion (`error[E0005]`)

GPU hardware execution threads operate on fixed register allocations rather than dynamically growing call stacks:

```text
error[E0005]: recursion is not supported inside #[nam]
  --> src/main.rs:15:5
   |
 7 | fn recursive_call(
   | ^^^^^^^^^^^^^^^^^^ recursive call occurs here
 ...
15 |     recursive_call(space, global_input, output, tile_cache);
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ recursive call occurs here
   |
   = note: GPU nams execute on fixed registers without a dynamic call stack to
           allocate new data at runtime.
   = help: rewrite recursive algorithms using iterative loops (`for` / `while`).
```

---

## 5. Host Operating System Calls (`error[E0001]`)

GPU execution units are hardware accelerators isolated from the CPU's operating system kernel. Attempting to invoke file I/O, network sockets, thread spawning, or standard output inside a `#[nam]` is rejected:

```text
error[E0001]: unsupported CPU system operation inside #[nam]
  --> src/main.rs:15:5
   |
15 |     println!("{:?}", tile_cache);
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsupported CPU call occurs here
   |
   = note: GPU nams execute isolated on graphics silicon without access to CPU
           OS services (I/O, filesystem, threads).
   = help: perform CPU operations in the main thread and transfer data using
           `GpuVec` or `GpuParam`.
```

---

## Summary of Compiler Diagnostic Invariants

| Code | Diagnostic Title | Hardware Invariant Violated |
| :--- | :--- | :--- |
| **`E0001`** | Unsupported CPU system operation | GPU silicon has no host OS or syscall interface. |
| **`E0002`** | Dynamic heap allocation unsupported | No physical memory allocator exists across workgroups. |
| **`E0003`** | Explicit panic or unwinding assertion | Hardware has no stack unwinding or exception tables. |
| **`E0004`** | Unsupported data type on GPU | Types larger than 64-bit (`u128`, `i128`) lack hardware registers. |
| **`E0005`** | Recursion is not supported | Threads operate on fixed register footprints without call stacks. |
| **`E0006`** | Inline assembly not supported | CPU assembly instructions cannot execute on GPU pipelines. |
| **`E0007`** | Mutable global state not permitted | Static mutable variables cause unresolvable multi-workgroup races. |
| **`E0010`** | Dynamic dispatch unsupported | Hardware requires direct branching; no vtables permitted. |
