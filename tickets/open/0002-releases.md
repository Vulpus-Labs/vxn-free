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

- [ ] A release workflow: universal macOS bundles signed with a Developer
      ID and notarized; Windows and Linux `.clap` files; zipped per
      platform, attached to the release.
- [ ] Signing secrets documented (names only) in the README's release
      section; nothing secret in the repo.
- [ ] A dry run on a pre-release tag.

## Notes

`cargo silkscreen bundle <package> --universal --sign --notarize` does the
macOS part.
