#!/usr/bin/env fish

docker build -t deno-edge:latest .; or exit 1
flyctl deploy --local-only; or exit 1
