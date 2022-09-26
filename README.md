# OpenEdge

An open source serverless edge runtime for JavaScript.

Built with deno_core, Rust, and V8.

## Deploy on fly.io

```sh
docker build -t openedge:latest .
flyctl deploy --local-only
flyctl regions add sea ord maa dfw fra syd
```
