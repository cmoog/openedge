Deno.serve(
  () =>
    new Response(`goodbye from openedge running in ${Deno.env.get("REGION")}`),
  {
    hostname: "0.0.0.0",
    port: Deno.env.get("PORT"),
  },
);
