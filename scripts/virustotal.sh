#!/bin/sh
# Uploads a file to VirusTotal, waits for the analysis and prints key=value lines for $GITHUB_OUTPUT:
# sha256, status (completed or timeout), microsoft (the Microsoft engine's category), microsoft_result
# (its detection name), flagged (every engine that called it malicious or suspicious) and detections
# ("<flagged> of <engines that gave a verdict>").
# Needs VT_API_KEY. Usage: virustotal.sh <file>
set -eu

file=$1
api=https://www.virustotal.com/api/v3
vt() { curl -sSf --retry 3 -H "x-apikey: $VT_API_KEY" "$@"; }

sha=$(sha256sum "$file" | cut -d' ' -f1)
echo "sha256=$sha"
# shortcut: the plain upload endpoint takes files up to 32 MB; past that, ask /files/upload_url first.
id=$(vt -F "file=@$file" "$api/files" | jq -r .data.id)

# The free API allows four requests a minute, so one poll every 30 seconds stays well under it.
status=queued
report=
for _ in $(seq 1 40); do
  sleep 30
  report=$(vt "$api/analyses/$id")
  status=$(printf '%s' "$report" | jq -r .data.attributes.status)
  [ "$status" = completed ] && break
done
if [ "$status" != completed ]; then
  echo "status=timeout"
  exit 0
fi

echo "status=completed"
printf '%s' "$report" | jq -r '.data.attributes.results as $r
  | "microsoft=\($r.Microsoft.category // "missing")",
    "microsoft_result=\($r.Microsoft.result // "")",
    "flagged=\([$r[] | select(.category == "malicious" or .category == "suspicious")
      | "\(.engine_name): \(.result)"] | join("; "))",
    "detections=\([$r[] | select(.category == "malicious" or .category == "suspicious")] | length) of \([$r[]
      | select(.category == "malicious" or .category == "suspicious" or .category == "undetected" or .category == "harmless")] | length)"'
