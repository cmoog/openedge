export default {
  fetch(_req) {
    return new Response(
      `hello from openedge running in ${Deno.env.get("REGION")}\n`,
    );
  },
};
