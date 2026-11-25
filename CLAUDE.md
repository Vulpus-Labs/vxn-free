# vxn-free: working conventions

vxn-free holds Vulpus Labs' free CLAP effects (Beverley, Glow-Up;
Charm is shelved, ADR 0002). It is public, under MIT OR Apache-2.0. It
is built on the public silkscreen library (<https://github.com/Vulpus-Labs/silkscreen>, checked out at
`~/src/silkscreen`).

## Public repo rules

- Nothing from private Vulpus Labs repos is named or linked here: no
  private crate, file, ticket or ADR references. Algorithms are ported and
  described in this repo's own words.
- No Voltage Modular skin art, nor art derived from it: faceplates and
  controls are drawn from scratch as SVG.
- Commits use the user's GitHub noreply identity (set in this checkout's
  git config).

## Process

- Decisions live in `adrs/NNNN-slug.md`, one per topic. A change that
  alters a decision updates or supersedes the ADR in the same change.
- Work is tracked in `epics/` and `tickets/` (see `tickets/README.md`).
  Work tickets in dependency order (`depends:` frontmatter). When a
  ticket's acceptance criteria are met, tick them, add a
  `## Close-out (YYYY-MM-DD)` section and `git mv` it to `tickets/closed/`.
  Close an epic the same way when its last ticket closes.
- Commit per ticket, with the ticket number in the subject.

## silkscreen

- Every silkscreen crate comes from one git tag in the root `Cargo.toml`.
- Library changes are made in `~/src/silkscreen`, following its own
  CLAUDE.md, released as a new tag there, then the tag is bumped here.
- Both repos pin the same toolchain (`rust-toolchain.toml`): snapshots
  depend on it. Bump them together.

## Verification

CI (`.github/workflows/ci.yml`) runs these; run them before committing.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p vxn-free-dsp --target wasm32-unknown-unknown
```

Bundles are built with silkscreen's tool, installed from the same tag as
the crates:
`cargo install --locked --git https://github.com/Vulpus-Labs/silkscreen --tag v0.3.1 silkscreen-tool`,
then `cargo silkscreen bundle <package>`.

## Code

- DSP is measured before it is changed: quantify aliasing and CPU with a
  scratch harness, and keep the numbers in the ticket.
- Engine code depends only on `silkscreen-bridge` and this repo's DSP
  crate. It must not allocate or lock.
- Prefer generated, typed code over runtime lookup. Build-time errors must
  name the offending element and the fix.
- The real FFT in `vxn-free-dsp` is a copy of public Patches'
  (`~/src/patches`, `patches-dsp/src/fft.rs`). A fix made to one copy is
  made to the other.
- Bit-exact tests compare outputs within a run, for example the same
  input split into different block sizes, never against stored values.
  The FFT picks a SIMD path (AVX2+FMA, NEON or scalar) at runtime, and
  the paths round differently.
