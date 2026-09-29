#!/usr/bin/env bash
for f in build/*; do
  rm -f "$f"
done

purge() {
  rm -rf cache
}
