# Ember EQ 🍂

A cozy six-band parametric equalizer from **Cove Labs**, written in Rust.

> Software made somewhere warm.

## v0.1

- 6 EQ bands
- Bell, low shelf, high shelf, low-cut and high-cut filters
- Frequency, gain and Q controls
- Per-band enable
- Master bypass
- Output gain
- Stereo and mono processing
- VST3 + CLAP exports
- Autumn-inspired interface
- Windows installer setup

## Build

```powershell
cargo test
cargo xtask bundle ember_eq --release
```

The bundled plugin will be written under `target/bundled/`.

## Status

Ember EQ is early in development. The EQ DSP is functional; the live response curve, draggable EQ nodes and spectrum analyser are planned next.

## License

GPL-3.0-or-later. See `LICENSE` and review the licensing of the plugin framework/VST3 bindings before redistribution.
