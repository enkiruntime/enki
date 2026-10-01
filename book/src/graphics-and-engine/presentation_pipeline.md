# Real-Time Presentation (`flow.present`)

Traditional graphics programming enforces a strict separation between compute pipelines and rendering pipelines. Drawing pixels on screen typically requires setting up render passes, vertex buffers, rasterizers, and fragment shaders.

Enki unifies this through **direct buffer presentation**: you write pixels into a standard contiguous VRAM buffer (`GpuVec<u32>`) using a compute nam, and queue it directly to the display window's swapchain in a single statement.

---

## 1. Adding Dependencies

To build a real-time windowed application, add `enki-gpu` and `glfw` to your project:

```bash
cargo add enki-gpu glfw
```

Your `Cargo.toml` dependencies should include:

```toml
[dependencies]
enki-gpu = "0.1"
glfw = "0.59"
```

---

## 2. Window Setup & The `NoApi` Invariant

Vulkan manages swapchain images directly through native platform display surfaces. Therefore, you must explicitly instruct GLFW **not** to create an OpenGL context:

```rust
let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW");

// MANDATORY: Disable OpenGL context creation
glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
glfw.window_hint(glfw::WindowHint::Resizable(true));

let (mut window, events) = glfw
    .create_window(WIDTH, HEIGHT, "Enki Display", glfw::WindowMode::Windowed)
    .expect("Failed to create GLFW window");

// Enable polling for resize and keyboard events
window.set_framebuffer_size_polling(true);
window.set_key_polling(true);
```

### Passing the Window to Enki
Enki's windowed initializer expects an atomic reference counted (`Arc<W>`) window instance implementing `raw_window_handle::HasWindowHandle` and `HasDisplayHandle`. 

Because GLFW's `Window` implements these traits natively, pass it directly:

```rust
let window_handle = Arc::new(window);

// Initialize Enki in windowed mode
let enki = Enki::init_windowed(window_handle.clone(), WIDTH, HEIGHT);
```

---

## 3. The Complete, Runnable Application

Below is a complete, self-contained `src/main.rs` that renders an animated real-time procedural color plasma at 60 FPS and handles dynamic window resizing cleanly without mutable borrowing conflicts:

```rust
use enki::*;
use glfw::{Action, Context, Key, WindowEvent, WindowHint, WindowMode};
use std::sync::Arc;
use std::time::Instant;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

// 1. Declare the compute nam that generates pixel colors
#[nam]
fn render_plasma(space: &Space, pixel: &mut u32, time: f32) {
    if !space.in_bounds_xy() {
        return;
    }

    let (u, v) = space.uv();

    // Compute animated color waves
    let r = ((u * 10.0 + time).sin() * 0.5 + 0.5);
    let g = ((v * 10.0 - time * 1.5).cos() * 0.5 + 0.5);
    let b = (((u + v) * 8.0 + time * 2.0).sin() * 0.5 + 0.5);

    // Pack RGB floats into 32-bit presentation integer (0xAARRGGBB)
    *pixel = space.set_rgb_color(r, g, b);
}

fn main() {
    // 2. Initialize GLFW with NoApi
    let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW");
    glfw.window_hint(WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
    glfw.window_hint(WindowHint::Resizable(true));

    let mut current_width = WIDTH;
    let mut current_height = HEIGHT;

    let (mut window, events) = glfw
        .create_window(current_width, current_height, "Enki Plasma", WindowMode::Windowed)
        .expect("Failed to create window");

    window.set_framebuffer_size_polling(true);
    window.set_key_polling(true);

    let window_handle = Arc::new(window);

    // 3. Initialize Enki windowed runtime
    let enki = Enki::init_windowed(window_handle.clone(), current_width, current_height);

    // 4. Allocate the physical VRAM framebuffer
    let mut screen = gpu_vec![0u32; (current_width * current_height) as usize];

    let start_time = Instant::now();
    let mut is_running = true;

    // 5. Main presentation loop
    while is_running && !window_handle.should_close() {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            match event {
                WindowEvent::Close => is_running = false,
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => is_running = false,
                _ => {}
            }
        }

        // Handle dynamic swapchain recreation if window dimensions changed
        let (fb_w, fb_h) = window_handle.get_framebuffer_size();
        if fb_w > 0 && fb_h > 0 && (fb_w as u32 != current_width || fb_h as u32 != current_height) {
            current_width = fb_w as u32;
            current_height = fb_h as u32;

            // Recreate Vulkan swapchain images
            let _ = enki.resize(current_width, current_height);

            // Reallocate VRAM screen buffer to match new extent
            screen = GpuVec::zeroed((current_width * current_height) as usize);
        }

        let time = start_time.elapsed().as_secs_f32();

        // 6. Record and submit the compute-to-display flow
        enki.flow(|flow| {
            render_plasma.run(
                &Space::gpu_xy(current_width as usize, current_height as usize),
                &mut screen,
                GpuParam::new(time),
            );

            // Queue buffer for direct presentation
            flow.present(&screen);
        });
    }
}
```

Run the application:

```bash
cargo run
```

A window will appear rendering an animated procedural plasma directly from the GPU compute cores at your monitor's native refresh rate.

---

## 4. The Presentation Lifecycle

When `flow.present(&screen)` is recorded:

```text
Host CPU Flow Recording
┌──────────────────────────────────────┐
│ render_plasma.run(&space, &mut screen) │ ──► Compute writes pixels into VRAM
├──────────────────────────────────────┤
│ flow.present(&screen)                 │ ──► Schedules hardware copy: Buffer ──► Swapchain Image
└──────────────────────────────────────┘
                   │
                   ▼ (Queue Submission)
Vulkan Hardware Timeline
──[Compute Shader Finishes]──► [vkCmdCopyBufferToImage] ──► [vkQueuePresentKHR] ──► Display
```

1. **Copy Command:** The engine injects an asynchronous transfer command (`vkCmdCopyBufferToImage`) from the `GpuVec` into the current swapchain image.
2. **Synchronization:** The engine automatically coordinates the timeline semaphore with the swapchain's image-acquired and render-finished binary semaphores.
3. **Atomic Display:** The frame is presented via `vkQueuePresentKHR` once memory transfers complete.

---

## 5. Dimension Verification (`error[E1008]`)

Swapchain presentation requires an exact 1:1 pixel mapping. Attempting to present a buffer that contains fewer elements than the active window extent (`width * height`) halts execution immediately:

 ```
 error[E1008]: GpuVec capacity is smaller than the requested space domain
   --> src/main.rs:88:27
    |
 88 |             render_plasma.run(
    |                           ^^^^ container contains fewer elements than required space cells
    |
    = note: each cell in `space` expects exclusive 1:1 access to its
            corresponding element; extra threads would access
            out-of-bounds memory.
    = note: argument 1 (`GpuVec<u32>`) contains only 1439999 elements
    = note: space execution (dimensions: 1600x900x1) requires at least
            1440000 elements (deficit of 1 elements)
    = help: resize the `GpuVec` using `.resize(...)` to cover all space
            cells, or adjust the space domain dimensions.
 ```
