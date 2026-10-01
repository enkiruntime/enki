# Hardware & Toolchain Setup

This chapter covers setting up your development environment, verifying hardware capabilities, and configuring the compilation toolchain for Enki.

---

## 1. Graphics Driver Verification

Enki targets **Vulkan 1.3** directly through physical device addresses. 

> **Note on Vulkan SDK:** You do **not** need to install the full LunarG Vulkan SDK to build or run Enki applications. A standard, up-to-date graphics driver provided by your GPU vendor (NVIDIA, AMD, or Intel) with Vulkan 1.3 support is completely sufficient.

To verify that your active GPU driver supports Vulkan 1.3 and the required features, you can run the standard diagnostic tool:

```bash
vulkaninfo --summary
```

Ensure that:
- The reported `vulkanVersion` is at least `1.3.xxx`.
- 64-bit Buffer Device Addresses (`bufferDeviceAddress`) are supported.

---

## 2. Adding Enki to Your Project

Create a new binary project using Cargo:

```bash
cargo new my_enki_app
cd my_enki_app
```

Add the `enki-gpu` runtime to your dependencies:

```bash
cargo add enki-gpu
```

If you plan to perform vector or matrix mathematics, we also recommend adding an external algebraic crate such as `glam`:

```bash
cargo add glam
```

---

## 3. The Execution Workflow

GPU JIT compilation requires two critical compiler flags to lower Rust code onto graphics silicon:
1. `--emit=llvm-bc`: Generates the LLVM bitcode representation required by the JIT backend (`parsu`).
2. `opt-level = 2` & `codegen-units = 1`: Optimization is mandatory to flatten high-level Rust abstractions into native silicon primitives, while single code generation units ensure monolithic bitcode generation.

Enki provides **two distinct workflows** to manage these flags:

### Path A: Automatic Project Configuration (`cargo run`)

This is the standard and recommended workflow. On the very first execution of your project:

```bash
cargo run
```

Enki will detect that `.cargo/config.toml` is missing the required flags and will prompt you:

```text
warning: missing required compilation profile and LLVM bitcode for GPU JIT synthesis
  --> append configuration to `.cargo/config.toml`? [Y/n]
```

Pressing **`Enter`** (or typing `Y`) will automatically append the required profile blocks to `.cargo/config.toml` and re-run your application:

```toml
# --- Added by Enki for GPU JIT compilation ---
[profile.dev]
opt-level = 2
codegen-units = 1

[profile.dev.package."*"]
opt-level = 2

[profile.release]
opt-level = 2
codegen-units = 1

[profile.release.package."*"]
opt-level = 2

[target.'cfg(all())']
rustflags = ["--emit=llvm-bc"]
# ---------------------------------------------
```

From this point forward, you can build and run using standard `cargo run` and `cargo run --release`.

---

### Path B: Non-Intrusive Execution (`enki run`)

If you prefer not to let the runtime modify your project's `.cargo/config.toml` file, you can use the official CLI runner, `cargo-enki`.

Install the CLI tool:

```bash
cargo install cargo-enki
```

Now, execute your project using the `enki` subcommand:

```bash
cargo enki run
# or directly:
enki run
```

The CLI tool automatically injects the necessary compiler flags and optimization profiles via environment variables during invocation, leaving your project directory completely untouched.

---

## 4. The Compiler Backend (`parsu`)

Enki compiles Rust functions into optimized GPU compute pipelines through its JIT backend, **`parsu`**.

On your first dispatch, Enki will automatically download the pre-compiled `parsu` binary matching your operating system from official releases and cache it locally. No manual compiler toolchain installation is required.

---

## Next Steps

With your environment configured, proceed to **[Your First Nam & Dual Debugging](hello_gpu.md)** to write and dispatch your first compute kernel.
