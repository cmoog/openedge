export default {
  fetch(_req, env) {
    return new Response(
      `goodbye from openedge running in ${env.REGION}`,
    );
  },
};
