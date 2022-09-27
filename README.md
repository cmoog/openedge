# OpenEdge

> **Warning** Not yet usable. Under development.

An open source serverless edge runtime for JavaScript and WebAssembly.

Built with deno_core, Rust, and V8.

## Test

```sh
docker run --rm -p 8080:8080 ghcr.io/cmoog/openedge:latest

# in another terminal
curl --header "host: hello.com" http://localhost:8080
curl --header "host: goodbye.com" http://localhost:8080
```

## Example

```javascript
export default {
  fetch(_req) {
    return new Response(
      `hello from openedge running in ${Deno.env.get("REGION")}\n`,
    );
  },
};
```

## Sandbox

The OpenEdge sandbox supports the
[WebPlatform APIs](https://deno.land/manual@v1.25.4/runtime/web_platform_apis).

### Permissions

Each worker runs in its own V8 isolate with restricted access to underlying
system APIs.

| Resource              | Scope                              | Example                        |
| --------------------- | ---------------------------------- | ------------------------------ |
| network access        | public internet, `localhost:$PORT` | `fetch("https://example.com")` |
| environment variables | "REGION", "PORT"                   | `Deno.env.get("REGION")`       |
| filesystem read       | none                               | -                              |
| filesystem write      | none                               | -                              |
| child process         | none                               | -                              |
| ffi                   | none                               | -                              |

## Deploy on [fly.io](https://fly.io)

```sh
flyctl apps create [app_name]
docker build -t openedge:latest .
flyctl deploy --app [app_name] --local-only
flyctl regions add sea ord maa dfw fra syd
```

## Closed Alternatives

Cloudflare Workers, Deno Deploy, AWS CloudFront Functions.
