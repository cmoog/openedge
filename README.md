# OpenEdge

An open source serverless edge runtime for JavaScript and WebAssembly.

Built with deno_core, Rust, and V8.

## Example

```javascript
Deno.serve(() => new Response(`hello from ${Deno.env.get("REGION")}`), {
  port: Deno.env.get("PORT"),
});
```

## Sandbox

The OpenEdge sandbox supports the [Deno](https://deno.land/api@v1.25.4) runtime
APIs, including
[WebPlatform APIs](https://deno.land/manual@v1.25.4/runtime/web_platform_apis).

### Permissions

Each worker runs in its own V8 isolate with restricted access to underlying
system APIs.

| Resource              | Scope                                 | Example                                                                          |
| --------------------- | ------------------------------------- | -------------------------------------------------------------------------------- |
| network access        | `localhost:$PORT` and public internet | `fetch("https://example.com")`, `Deno.serve(..., { port: Deno.env.get("PORT")})` |
| environemnt variables | `"PORT"`, `"REGION"`                  | `Deno.env.get("PORT")`                                                           |
| filesystem read       | none                                  | -                                                                                |
| filesystem write      | none                                  | -                                                                                |
| child process         | none                                  | -                                                                                |
| ffi                   | none                                  | -                                                                                |

## Deploy on [fly.io](https://fly.io)

```sh
flyctl apps create [app_name]
docker build -t openedge:latest .
flyctl deploy --app [app_name] --local-only
flyctl regions add sea ord maa dfw fra syd
```

## Closed Alternatives

Cloudflare Workers, Deno Deploy, AWS CloudFront Functions.
