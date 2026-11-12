#!/usr/bin/env bash
# Sums resident memory (MB) of every process whose command line matches a pattern,
# including WebView helpers. Usage: scripts/measure-rss.sh <pattern>
# Example: scripts/measure-rss.sh Termian    (legacy app + WebKit helpers)
#          scripts/measure-rss.sh termian-native
set -euo pipefail
pattern=${1:?usage: measure-rss.sh <pattern>}
ps -axo rss=,command= | grep -i -- "$pattern" | grep -v -e grep -e measure-rss \
  | awk '{ kb += $1; n++ } END { printf "%d processes, %.1f MB RSS\n", n, kb / 1024 }'
