#!/usr/bin/env bash
# Uploads a file to VirusTotal, waits for the analysis and prints key=value lines for $GITHUB_OUTPUT:
# sha256, status (completed or timeout), microsoft (the Microsoft engine's category), microsoft_result
# (its detection name) and flagged (every engine that called it malicious or suspicious).
# Needs VT_API_KEY. Usage: virustotal.sh <file>
set -euo pipefail

file=$1
api=https://www.virustotal.com/api/v3
auth=(-H "x-apikey: $VT_API_KEY")

sha=$(sha256sum "$file" | cut -d' ' -f1)
echo "sha256=$sha"
# shortcut: the plain upload endpoint takes files up to 32 MB; past that, ask /files/upload_url first.
id=$(curl -sSf --retry 3 "${auth[@]}" -F "file=@$file" "$api/files" | jq -r .data.id)

# The free API allows four requests a minute, so one poll every 30 seconds stays well under it.
status=queued
for _ in $(seq 1 40); do
  sleep 30
  report=$(curl -sSf --retry 3 "${auth[@]}" "$api/analyses/$id")
  status=$(jq -r .data.attributes.status <<<"$report")
  [ "$status" = completed ] && break
done
if [ "$status" != completed ]; then
  echo "status=timeout"
  exit 0
fi

echo "status=completed"
echo "microsoft=$(jq -r '.data.attributes.results.Microsoft.category // "missing"' <<<"$report")"
echo "microsoft_result=$(jq -r '.data.attributes.results.Microsoft.result // ""' <<<"$report")"
echo "flagged=$(jq -r '[.data.attributes.results[]
  | select(.category == "malicious" or .category == "suspicious")
  | "\(.engine_name): \(.result)"] | join("; ")' <<<"$report")"
