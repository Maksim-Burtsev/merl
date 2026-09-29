#!/usr/bin/env bash
# Never pass a ` to tar here.

archive() {
  tar -czf shop.tgz lib
}
