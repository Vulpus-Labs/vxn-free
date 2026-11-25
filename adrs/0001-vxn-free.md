# 0001: vxn-free, free CLAP effects on silkscreen

Status: accepted (2026-11-24)

## Context

Vulpus Labs has made effect modules for Voltage Modular. Two of them,
Beverley (a bitcrusher) and Glow Up (a harmonic exciter, in its rebuilt
form), are worth having as plugins in any DAW, and silkscreen makes that a
small job. They are to be given away, with their source.

## Decision

1. **Repo.** `Vulpus-Labs/vxn-free` on GitHub, public, licensed MIT OR
   Apache-2.0 like silkscreen. It names and links nothing in private
   Vulpus Labs repos; ported algorithms are explained in its own words.
2. **Plugins.** Beverley (`labs.vulpus.beverley`) and Glow-Up
   (`labs.vulpus.glow-up`), vendor Vulpus Labs. Ids never change once
   released: hosts key projects on them. ADR 0002 adds Charm
   (`labs.vulpus.charm`), an original design rather than a port, since
   shelved; ADR 0003 proposes Wesley (`labs.vulpus.wesley`), Beverley in
   three bands.
3. **Targets.** CLAP and VST3, built with `cargo silkscreen bundle`
   (the VST3 through free-audio's clap-wrapper, `--vst3`; amended
   2026-11-24, ticket 0027). No web page and no standalone app for now;
   silkscreen offers both if wanted later.
4. **silkscreen.** Every silkscreen crate from one git tag (`v0.2.1`
   first), with the same pinned toolchain as silkscreen.
5. **Layout.**
   - `crates/vxn-free-dsp`: DSP the plugins share (filters, smoothing,
     the STFT and peak tracking Glow-Up needs). Builds for
     `wasm32-unknown-unknown`, so a web page stays possible.
   - The real FFT is copied from public Patches
     (<https://github.com/Vulpus-Labs/patches>, `patches-dsp/src/fft.rs`,
     MIT) into `vxn-free-dsp`. Its author relicenses the copy under
     MIT OR Apache-2.0. It is the only code taken from Patches, and this
     repo has no dependency on Patches. The copy keeps runtime SIMD
     dispatch (AVX2+FMA, NEON, scalar).
   - Per plugin, an editor/engine crate and a `-clap` crate, as
     silkscreen's template lays them out.
6. **Adaptation, not emulation.** The ports keep each module's sound and
   controls, with these changes:
   - Voltage scaling goes: audio is ±1.0, not ±5 V.
   - CV inputs and their AMT knobs go: host automation and modulation do
     that job.
   - Fixed sample-rate assumptions go: coefficients come from the host's
     rate.
   - Each plugin gains what a plugin needs and a module didn't, such as
     a dry/wet mix where it has none, decided in its ticket.
7. **No licensing.** Free plugins: no licence check, no unlock dialog.
8. **Art.** Faceplates and controls are original SVG. Nothing is taken
   from Voltage Modular's skins or art made with them.
9. **Releases.** Signed and notarized macOS bundles plus Windows and
   Linux builds, attached to GitHub releases.

## Consequences

- Glow-Up's analysis is written in `vxn-free-dsp`: the STFT framing,
  peak detection and partial tracking, on the copied FFT.
- Fixes to the FFT are made in both copies by hand. It is small and
  stable, so it is rarely touched.
- Patches is public, so naming and linking it is allowed under the
  public-repo rule.
- Without CV, modulated sweeps rely on the host; each plugin's
  parameters are smoothed so automation doesn't zipper.
