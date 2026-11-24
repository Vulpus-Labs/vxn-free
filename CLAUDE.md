# vxn-free: working conventions

vxn-free holds Vulpus Labs' free CLAP effects (Beverley, Glow-Up). It is
public, under MIT OR Apache-2.0. It is built on the public silkscreen
library (<https://github.com/Vulpus-Labs/silkscreen>, checked out at
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

## Code

- DSP is measured before it is changed: quantify aliasing and CPU with a
  scratch harness, and keep the numbers in the ticket.
- Engine code depends only on `silkscreen-bridge` and this repo's DSP
  crate. It must not allocate or lock.
- Prefer generated, typed code over runtime lookup. Build-time errors must
  name the offending element and the fix.
