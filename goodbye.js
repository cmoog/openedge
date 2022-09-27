export default {
  fetch(_req, _env) {
    return new Response(
      `goodbye from openedge running in ${Deno.env.get("REGION")}`,
    );
  },
};
