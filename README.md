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

## Editor controls

- Drag graph nodes to set frequency and gain. Cut filters move horizontally.
- Hold the left mouse button on a node or frequency/gain slider and turn the mouse wheel to change that band's Q. Wheel up narrows the band; wheel down broadens it.
- All six band cards use bounded columns, with scrolling when vertical space is limited.
- **Reset band** restores its default frequency, type, gain and Q, preserving its enabled state.
- **Flat EQ** disables all six bands and resets output to 0 dB. Enable individual bands to build a new curve.
- **A → B / B → A** switches between two editable settings. The first switch copies A into B. Comparison includes band settings and output gain, and lasts while the editor is open; the host saves the currently active settings.
- The response curve uses the host sample rate and includes output gain and master bypass.

## Possible next features

- Input/output spectrum analyser to spot resonances and see the effect of each adjustment.
- Band audition to help locate problem frequencies by ear.
- Output peak meters with a clip indicator for managing headroom.
- Named presets and preset import/export for reusing settings across sessions.
- Selectable cut slopes for more control over low/high frequency cleanup.

These are future ideas, not implemented features.

## License

GPL-3.0-or-later. See `LICENSE` and review the licensing of the plugin framework/VST3 bindings before redistribution.
