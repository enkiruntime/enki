# Internal Compiler Errors & Reporting (`error[E9999]`)

During the active **alpha phase (`v0.1`)**, the JIT compilation toolchain (`parsu`) undergoes continuous testing across diverse hardware architectures and complex Rust language constructs.

If the compiler encounters an unexpected invariant violation while lowering LLVM bitcode into GPU SPIR-V, it aborts execution with an **Internal Compiler Error (ICE)**.

---

## 1. What is an Internal Compiler Error (ICE)?

An ICE does not indicate a syntax error or a type-safety violation in your code. Rather, it signifies that the JIT compiler backend encountered an edge case it could not resolve internally (such as an unhandled LLVM intrinsic, an unexpected register footprint, or an unsupported optimization graph state).

When this happens, the compiler catches the internal state, packages the compilation failure metadata, and outputs diagnostic **`error[E9999]`**:

```text
error[E9999]: internal compiler error (ICE) in Parsu GPU backend (v0.1.0-alpha)
  --> src/main.rs:10:1
   |
10 | #[nam]
   | ^^^^^^ compiler hit an unexpected invariant condition
   |
   = note: this is a known limitation of the early alpha release on some architectures.
   = note: the compiler caught an internal failure and sealed the state into an encrypted token

  -----BEGIN ENKI CRASH FLIGHT RECORDER TOKEN-----
  eJy1V9tu2zYQfb9f8eAF... [SEALED DIAGNOSTIC PAYLOAD] ...bX2F1
  -----END ENKI CRASH FLIGHT RECORDER TOKEN-----

   = help: we would appreciate a bug report! Please open an issue and include your #[nam] code along with the diagnostic token below:
           https://github.com/enkiruntime/enki/issues
```

---

## 2. Reporting Compiler Issues

Because Enki is in active alpha, reporting these edge cases directly contributes to the stability and maturity of the platform.

If you encounter an `E9999` diagnostic:

1. **Isolate a Minimal Example:** Try to isolate the minimal `#[nam]` function that triggers the condition.
2. **Open a GitHub Issue:** Visit [github.com/enkiruntime/enki/issues](https://github.com/enkiruntime/enki/issues).
3. **Include the Full Diagnostic:** Copy and paste the entire terminal output, including the sealed token block.

The diagnostic token allows me to reconstruct the compiler phase, target nam, and invariant failure without requiring access to your proprietary code.
