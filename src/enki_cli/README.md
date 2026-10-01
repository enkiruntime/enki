# cargo-enki

CLI toolchain and runner for the Enki heterogeneous GPU compute platform.
Part of the [Enki](https://github.com/enkiruntime/enki) ecosystem.

## Installation

```bash
cargo install cargo-enki
```

## Usage

```bash
# Run with GPU JIT support
cargo enki run

# Or directly
enki run
enki run --release
```

## Crash Token Decoder

```bash
enki debug <TOKEN>
```
