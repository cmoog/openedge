export default {
  async fetch(_req, env) {
    const resp = await fetch("https://github.com/cmoog.keys");
    const keys = await resp.text();
    return new Response(
      `hello from openedge running in ${env.REGION}

------------
    keys
------------
  
${keys}
      `
    );
  },
};
