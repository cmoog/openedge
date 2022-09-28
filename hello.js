export default {
  fetch(_req, env) {
    return new Response(
      `hello from openedge running in ${env.REGION}\n`,
    );
  },
};
