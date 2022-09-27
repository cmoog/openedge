Deno.serve(
  () =>
    new Response(`hello from openedge running in ${Deno.env.get("REGION")}\n`),
  {
    hostname: "0.0.0.0",
    port: Deno.env.get("PORT"),
  },
);
