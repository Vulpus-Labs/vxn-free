# 0003: Manuals, typeset from the plugins' own code

Status: accepted (2026-11-24)

## Context

Each plugin needs a manual for players. The user (2026-11-24) asked for
one built as Vulpus Labs builds its instruments' manuals: a full PDF, with
graphs generated from the audio engine rather than drawn by hand.

## Decision

1. **Typst, one template.** `docs/manual/vulpus-manual.typ` sets the page,
   the type (Avenir Next and Charter, which macOS has), labelled faces and
   plots. Each manual imports it as `/docs/manual/vulpus-manual.typ` and
   is compiled with `typst compile --root <repo>`. A manual lives beside
   its plugin, in `crates/<plugin>/manual/`: `build.sh`, the `.typ`, and
   `data/`.
2. **Data from the plugin's code.** An example in the plugin's crate
   (`cargo run --release -p <plugin> --example manual -- DIR`) writes
   everything the manual shows:
   - the faceplate as the editor draws it, at 2×, through
     `silkscreen-harness`, and `positions.json`, every control's box from
     the generated layout, for the numbered badges;
   - measurements, as CSV, taken by running the engine as a host does
     (params set, the engine reset, a signal processed, the output
     analysed), and tables read from the param table and its tapers.
   Numbers in the prose are the code's constants or these measurements.
3. **What is committed.** The CSV and JSON in `data/` are committed, so a
   review shows a measurement that moved. The faces and the PDF are not:
   `build.sh` regenerates them. The PDF is published on vulpuslabs.com
   (`/manuals/<plugin>/`), linked from the plugin's product page and from
   this repository's README.

## Consequences

- A manual is built on a Mac (the fonts), not in CI.
- A change to a control or to the DSP shows up as a changed CSV; rebuild
  the manual with it, and check the prose that quotes it.
