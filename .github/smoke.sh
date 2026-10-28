#!/usr/bin/env bash
# Starts the built desktop app and requires it to keep running for 12 seconds.
# usage: smoke.sh /path/to/app
"$1" > smoke.log 2>&1 &
pid=$!
sleep 12
if kill -0 "$pid" 2>/dev/null; then
  echo "app still running after 12 s"
  kill "$pid"
  exit 0
fi
wait "$pid"
echo "app exited early with code $?"
cat smoke.log
exit 1
