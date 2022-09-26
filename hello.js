Deno.serve(
  async () => {
    const keys = await (await fetch("https://github.com/cmoog.keys")).text();
    return new Response(
      `hello from openedge running in ${Deno.env.get("REGION")}\n${keys}`,
    );
  },
  {
    hostname: "0.0.0.0",
    port: Deno.env.get("PORT"),
  },
);
