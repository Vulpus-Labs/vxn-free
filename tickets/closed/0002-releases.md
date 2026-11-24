---
id: "0002"
title: "Release builds on GitHub releases"
priority: medium
created: 2026-11-24
epic: E001
depends: ["0005"]
---

## Summary

Tagging `vX.Y.Z` builds every plugin and attaches it to a GitHub release.

## Acceptance criteria

- [x] A release workflow: universal macOS bundles signed with a Developer
      ID and notarized; Windows and Linux `.clap` files; zipped per
      platform, attached to the release.
- [x] Signing secrets documented (names only) in the README's release
      section; nothing secret in the repo.
- [x] A dry run on a pre-release tag (done as a real release instead: see below).

## Notes

`cargo silkscreen bundle <package> --universal --sign --notarize` does the
macOS part.

## Progress (2026-11-24)

- `.github/workflows/release.yml`: on a `v*` tag or by hand, never on
  pull requests. macOS (universal, `macos-15`), Windows (`windows-2025`)
  and Linux (`ubuntu-24.04`) build each package in `PACKAGES` with
  silkscreen-tool, installed from the tag the root `Cargo.toml` pins.
  clap-validator 0.4.1 checks every bundle, and each is zipped per
  platform as `<Bundle>-<version>-<platform>.clap.zip` (`ditto` on macOS,
  so the signature and stapled ticket survive). A tag must match the
  workspace version. Its zips go to a draft release, marked as a
  pre-release when the tag has a hyphen.
- Signing follows silkscreen's `--notarize`: the certificate goes into
  the job's own keychain, deleted at the end; the notary API key is
  written beside it; `SILKSCREEN_SIGN_IDENTITY` is read from the
  keychain. Without `MACOS_CERT_P12` the macOS build is unsigned, with a
  warning.
- README: a release section naming the five secrets.
- The secrets are organisation secrets, shared with this repository.
- A run by hand on main (run 45409480558, 58555bd) passed on all three
  platforms, and clap-validator passed on each:
  - macOS: signed, then notarized (status Accepted) and stapled, and
    `spctl` assessed it as "Notarized Developer ID".
  - The zip, downloaded and unpacked with `ditto`, checked again here:
    `spctl` accepted it, the stapled ticket validates, the signature has
    the hardened runtime and team 357M9JY6XD, and the binary is
    x86_64 + arm64.
  - The Windows and Linux zips each hold `Beverley.clap`.
- The first CI run on GitHub (all three jobs) passed.

## Close-out (2026-11-24)

The pre-release dry run became the first real release:
downloads.vulpuslabs.com serves only plain version folders (0.1.0, not
0.1.0-rc.1), so the user chose to tag `v0.1.0` straight away.

- The `v0.1.0` tag (run 45414448454) passed the tag/version check and
  built all three platforms, each clap-validated. It made a draft
  release, not a pre-release, holding `beverley-0.1.0-{macos,windows,
  linux}.clap.zip`. Zip names are lower case, as the download server
  names them.
- The draft's macOS bundle is notarized (stapled, Gatekeeper accepts it,
  "Notarized Developer ID") and universal.
- Published to `https://downloads.vulpuslabs.com/beverley/0.1.0/` with
  a `SHA256SUMS`, by the procedure in the download host's notes (rsync,
  `--ignore-existing`). The files fetched back match. A quarantined
  download of the macOS zip passes Gatekeeper.
- Linked from vulpuslabs.com's new Beverley page (`/products/beverley/`)
  and its "Free effects" sections (vulpus-labs-site 1fe522e).
- The GitHub release is still a draft. The site links to the download
  server, so publishing the draft is optional.
