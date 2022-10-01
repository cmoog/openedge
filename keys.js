export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (url.pathname.length < 2) {
      return Response.redirect("https://keys.edge.cmoog.dev/cmoog");
    }
    const username = usernameMatcher.exec(url.pathname.slice(1));
    if (!username) {
      return new Response(`invalid username "${url.pathname.slice(1)}"\n`, {
        status: 400,
      });
    }
    const resp = await fetch(`https://github.com/${username}.keys`);
    if (!resp.ok) {
      return new Response(`failed to fetch keys for username "${username}"\n`, {
        status: resp.status,
      });
    }
    const keys = await resp.text();
    return new Response(
      `hello from openedge running in ${env.REGION}
-----------------------------------
github public ssh keys for
${username}
-----------------------------------
	
${keys}
		`,
    );
  },
};

const usernameMatcher = /^[a-z\d](?:[a-z\d]|-(?=[a-z\d])){0,38}$/i;
