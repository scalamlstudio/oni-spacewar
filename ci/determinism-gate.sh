#!/usr/bin/env bash
# Determinism gate for the `sim` crate (TAKOAI-21). Run from the repo root.
#
#   1. Headless SyncTest, 4 players, 2 simulated minutes: must reach the frame
#      limit with 0 checksum mismatches.
#   2. Negative control (--inject-desync): must be caught — non-zero exit AND a
#      reported mismatch. A clean pass here means the checker is blind.
#
# Relies on the client's run report ("synctest mismatches : N") and its exit
# code (1 on the first mismatch). Build first: cargo build --release --locked.
set -uo pipefail

BIN=${BIN:-target/release/oni-spacewar}
MINUTES=${MINUTES:-2}
FRAMES=$((MINUTES * 3600))
LOGS=${LOGS:-target/determinism}
mkdir -p "$LOGS"
fail=0

mismatches() { sed -n 's/^synctest mismatches *: *\([0-9]*\).*/\1/p' "$1" | tail -n1; }
frames_reached() { sed -n 's/^sim frames reached *: *\([0-9]*\).*/\1/p' "$1" | tail -n1; }

echo "== SyncTest: ${MINUTES} sim-min, 4 players =="
"$BIN" synctest --headless --minutes "$MINUTES" --players 4 | tee "$LOGS/synctest.log"
code=${PIPESTATUS[0]}
m=$(mismatches "$LOGS/synctest.log")
f=$(frames_reached "$LOGS/synctest.log")
if [[ $code -ne 0 || "$m" != "0" || -z "$f" || "$f" -lt "$FRAMES" ]]; then
  echo "::error::SyncTest failed (exit $code, mismatches '${m}', frames '${f}' of $FRAMES)"
  fail=1
else
  echo "SyncTest OK: $f frames, 0 mismatches"
fi

echo "== Negative control: --inject-desync must be detected =="
"$BIN" synctest --headless --minutes 1 --players 2 --inject-desync | tee "$LOGS/negative.log"
code=${PIPESTATUS[0]}
m=$(mismatches "$LOGS/negative.log")
if [[ $code -eq 0 || -z "$m" || "$m" -eq 0 ]]; then
  echo "::error::Negative control NOT detected (exit $code, mismatches '${m}') — the desync checker is not firing"
  fail=1
else
  echo "Negative control OK: injected desync caught ($m mismatch event(s), exit $code)"
fi

exit $fail
