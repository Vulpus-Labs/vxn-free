#!/bin/sh
# Wesley's manual: draw the faceplate and measure the engine, then typeset the PDF.
#   ./build.sh            regenerate data/, then typeset
#   ./build.sh --no-data  typeset from the data already in data/ (faces/ must exist)
# Needs Typst (https://typst.app) and macOS's Avenir Next and Charter.
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
typst=$(command -v typst || echo "$HOME/.local/bin/typst")

if [ "$1" != "--no-data" ]; then
  (cd "$repo" && cargo run --release -q -p wesley --example manual -- "$here/data")
fi

"$typst" compile --root "$repo" "$here/wesley-manual.typ" "$here/Wesley-Manual.pdf"
echo "wrote $here/Wesley-Manual.pdf"
