#!/usr/bin/env bash
# Usage: release-files.sh <tag> <notes file> <download base URL> <out dir>
# Run from the repo root after `tauri build`. Collects the release files and
# writes latest.json for the in-app updater. The .sig files are not copied;
# their contents go into latest.json.
set -euo pipefail

if [ "$#" -ne 4 ]; then
  echo "usage: $0 <tag> <notes file> <download base URL> <out dir>" >&2
  exit 2
fi

tag=$1
notes=$2
base=$3
out=$4
v="${tag#v}"

deb="target/release/bundle/deb/RepoBell_${v}_amd64.deb"
appimage="target/release/bundle/appimage/RepoBell_${v}_amd64.AppImage"

for f in "$deb" "$deb.sig" "$appimage" "$appimage.sig" "$notes"; do
  [ -f "$f" ] || { echo "missing input file: $f" >&2; exit 1; }
done

mkdir -p "$out"
cp "$deb" "$out/"
cp "$appimage" "$out/"
cp "$deb" "$out/RepoBell_amd64.deb"
cp "$appimage" "$out/RepoBell_amd64.AppImage"

jq -n \
  --arg version "$v" \
  --rawfile notes "$notes" \
  --arg pub_date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg deb_url "$base/RepoBell_${v}_amd64.deb" \
  --rawfile deb_sig "$deb.sig" \
  --arg app_url "$base/RepoBell_${v}_amd64.AppImage" \
  --rawfile app_sig "$appimage.sig" \
  '{
    version: $version,
    notes: ($notes | sub("\\s+$"; "")),
    pub_date: $pub_date,
    platforms: {
      "linux-x86_64-deb": { url: $deb_url, signature: ($deb_sig | sub("\\s+$"; "")) },
      "linux-x86_64-appimage": { url: $app_url, signature: ($app_sig | sub("\\s+$"; "")) }
    }
  }' > "$out/latest.json"
