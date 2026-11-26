# vxn-free

Free audio effect plugins from Vulpus Labs, built with
[silkscreen](https://github.com/Vulpus-Labs/silkscreen), as CLAP and VST3.

- **Beverley**: a bitcrusher that adds harmonic grit rather than plain
  stair-steps. Gain or automatic gain, a gamma curve, fractional bit depth,
  smoothing between quantisation levels, and symmetric or asymmetric crush
  ([manual](https://vulpuslabs.com/manuals/beverley/),
  [downloads](https://vulpuslabs.com/products/beverley/)).
- **Wesley**: Beverley in three bands. Two crossovers split the input into
  LOW, MID and HIGH, which sum flat; each band has Beverley's controls, its
  own level and mix, and a solo. The bass is left clean by default
  ([manual](https://vulpuslabs.com/manuals/wesley/),
  [downloads](https://vulpuslabs.com/products/wesley/)).
- **Glow-Up**: a harmonic exciter. It finds the strongest partials in the
  input and boosts what is already there at two ratios of each, each set
  as a multiplier over a divisor (an octave and a fifth above, by default),
  with resonance up to self-oscillation
  ([manual](https://vulpuslabs.com/manuals/glow-up/),
  [downloads](https://vulpuslabs.com/products/glow-up/)).

Beverley and Glow-Up are adaptations of Vulpus Labs modules for Voltage
Modular.

**Status:** Beverley and Wesley are released (0.2.0), and Glow-Up
(0.3.0). The plan is in
[`epics/`](epics/) and [`tickets/`](tickets/), the decisions in
[`adrs/`](adrs/).

## Manuals

Each plugin's manual is a PDF typeset with [Typst](https://typst.app) from
its own code: `crates/<plugin>/manual/build.sh` draws the faceplate and
measures the engine (`cargo run -p <plugin> --example manual`), then
typesets `crates/<plugin>/manual/<plugin>-manual.typ` (ADR 0003). The
built manuals are on vulpuslabs.com.

## Releases

`.github/workflows/release.yml` builds every plugin as a CLAP and a
VST3 on macOS (one universal bundle for Apple silicon and Intel), Windows
and Linux, checks each CLAP with clap-validator and each VST3 with
Steinberg's validator, and zips each per platform. The VST3s come from
free-audio's clap-wrapper, through `cargo silkscreen bundle --vst3`. It runs when a
`v*` tag is pushed, and by hand from the Actions tab; never on pull
requests. A tag must match the workspace version in `Cargo.toml`
(`v0.1.0`, or `v0.1.0-rc.1` for a pre-release). A tag's zips land in a
draft GitHub release, which is published by hand.

macOS bundles are signed with a Developer ID and notarized when these
secrets are set, on the repository or shared to it from the
organisation; without them the bundles are built unsigned, with a
warning:

| Secret | Holds |
|---|---|
| `MACOS_CERT_P12` | The Developer ID Application certificate and its private key, as a `.p12`, base64-encoded |
| `MACOS_CERT_PASSWORD` | The `.p12`'s password |
| `NOTARY_KEY_P8` | An App Store Connect API key, the text of its `.p8` |
| `NOTARY_KEY_ID` | That key's id |
| `NOTARY_ISSUER_ID` | Its issuer id |

Windows and Linux builds aren't signed.

VST is a registered trademark of Steinberg Media Technologies GmbH. The
VST3 builds contain the VST3 SDK (MIT), clap-wrapper (MIT) and the CLAP
headers (MIT).

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT), at your option.
