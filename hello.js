Deno.serve(() => new Response(`hello from deno-edge running in ${Deno.env.get("REGION")}`), {
  hostname: "0.0.0.0",
  port: Deno.env.get("PORT"),
});
