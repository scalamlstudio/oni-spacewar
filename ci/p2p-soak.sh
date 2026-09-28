#!/usr/bin/env bash
# Local P2P soak (TAKOAI-23). Run from the repo root after
# `cargo build --release`. Needs `matchbox_server` 0.14 on PATH (or MATCHBOX).
#
#   ci/p2p-soak.sh                 4 peers, 5 min, 50 ms one-way (±5) + 2% loss
#   PEERS=2 MINUTES=1 ci/p2p-soak.sh
#   NEGATIVE=1 ci/p2p-soak.sh      peer 1 injects a desync: must be caught
#
# All peers run headless with bots on this machine and connect through a
# signaling server started here. Passes when every peer reaches the frame
# limit with 0 desyncs and exits 0 (or, with NEGATIVE=1, when every peer
# reports a desync and exits non-zero). Not run in CI: it takes real time.
set -uo pipefail

BIN=${BIN:-target/release/oni-spacewar}
MATCHBOX=${MATCHBOX:-matchbox_server}
PEERS=${PEERS:-4}
MINUTES=${MINUTES:-5}
NEGATIVE=${NEGATIVE:-0}
LOGS=${LOGS:-target/p2p-soak}
NET=${NET:---delay-ms 50 --jitter-ms 5 --loss 0.02}
FRAMES=$(awk "BEGIN { print int($MINUTES * 3600) }")
mkdir -p "$LOGS"

"$MATCHBOX" >"$LOGS/matchbox.log" 2>&1 &
server=$!
trap 'kill $server 2>/dev/null' EXIT
sleep 1

room="ws://127.0.0.1:3536/soak-$$?next=$PEERS"
pids=()
for i in $(seq 1 "$PEERS"); do
  extra=()
  [[ $NEGATIVE == 1 && $i == 1 ]] && extra=(--inject-desync)
  # shellcheck disable=SC2086
  "$BIN" p2p --headless --bot --players "$PEERS" --minutes "$MINUTES" \
    $NET --room "$room" ${extra[@]+"${extra[@]}"} >"$LOGS/peer$i.log" 2>&1 &
  pids+=($!)
done

fail=0
for i in $(seq 1 "$PEERS"); do
  wait "${pids[$((i - 1))]}"
  code=$?
  log="$LOGS/peer$i.log"
  d=$(sed -n 's/^p2p desyncs detected *: *\([0-9]*\).*/\1/p' "$log" | tail -n1)
  f=$(sed -n 's/^sim frames reached *: *\([0-9]*\).*/\1/p' "$log" | tail -n1)
  r=$(sed -n 's/^advances.*rollbacks: *\([0-9]*\).*/\1/p' "$log" | tail -n1)
  loss=$(sed -n 's/^net-emu: .*(\(.*\))$/\1/p' "$log" | tail -n1)
  echo "peer $i: exit $code, frames ${f:-?}/$FRAMES, desyncs ${d:-?}, rollbacks ${r:-?}, loss ${loss:-?}"
  if [[ $NEGATIVE == 1 ]]; then
    [[ $code -ne 0 && -n "$d" && "$d" -gt 0 ]] || fail=1
  else
    [[ $code -eq 0 && "$d" == "0" && -n "$f" && "$f" -ge $((FRAMES - 600)) ]] || fail=1
  fi
done

if [[ $fail -ne 0 ]]; then
  echo "P2P soak FAILED (logs in $LOGS)"
else
  echo "P2P soak OK"
fi
exit $fail
