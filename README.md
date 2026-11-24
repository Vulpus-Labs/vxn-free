# vxn-free

Free audio effect plugins from Vulpus Labs, built with
[silkscreen](https://github.com/Vulpus-Labs/silkscreen). CLAP only.

- **Beverley**: a bitcrusher that adds harmonic grit rather than plain
  stair-steps. Gain or automatic gain, a gamma curve, fractional bit depth,
  smoothing between quantisation levels, and symmetric or asymmetric crush.
- **Glow-Up**: a harmonic exciter. It finds the strongest partials in the
  input and boosts what is already there a fifth, an octave and two octaves
  above them, with resonance up to self-oscillation.

Both are adaptations of Vulpus Labs modules for Voltage Modular.

**Status:** in development; no release yet. The plan is in
[`epics/`](epics/) and [`tickets/`](tickets/), the decisions in
[`adrs/`](adrs/).

## Releases

`.github/workflows/release.yml` builds every plugin on macOS (one
universal bundle for Apple silicon and Intel), Windows and Linux, checks
each with clap-validator, and zips each per platform. It runs when a
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

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT), at your option.
