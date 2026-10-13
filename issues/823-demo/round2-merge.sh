#!/bin/bash
# Between the rounds main moves (a new timeout in app/config.py) and the author merges it, then pushes the fixes.
set -e
cd /tmp/823/parcel
git checkout -q main
sed -i '' 's/"TIMEOUT", "5"/"TIMEOUT", "10"/' app/config.py
git commit -qam "longer timeout"
git checkout -q fix/retries
git merge -q --no-edit main
${DEMO:-.}/round2.sh
