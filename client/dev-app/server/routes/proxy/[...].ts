// The dev relay: /proxy/<service>/<path...> forwarded to the voc service on its loopback port with the development
// credential and a fixed reader, the headers the console would set on every call. Nothing of a deployment's tokens
// stands here; the values come from the environment the host runs with.

const ADDRESS = process.env.BMO_DEV_VOC_ADDRESS ?? "http://127.0.0.1:13134";
const TOKEN = process.env.BMO_DEV_VOC_TOKEN ?? "";

// What of the service's answer rides back to the browser: its type, and how an attachment is to be shown or saved.
const PASSED = ["content-type", "content-disposition", "x-content-type-options"];

export default defineEventHandler(async (event) => {
  const rest = event.path.replace(/^\/proxy\/[^/]+/, "");

  const response = await fetch(`${ADDRESS}${rest}`, {
    method: event.method,
    headers: {
      authorization: `Bearer ${TOKEN}`,
      "owner-key": process.env.BMO_DEV_OWNER_KEY ?? "1",
      "owner-role": process.env.BMO_DEV_OWNER_ROLE ?? "member",
      ...(event.headers.get("content-type") ? { "content-type": String(event.headers.get("content-type")) } : {})
    },
    // The body rides through as the bytes the caller sent, as the console's relay does; reparsing it
    // here would rewrite what the service is asked to read.
    body: event.method === "GET" || event.method === "HEAD" ? undefined : (await readRawBody(event, false)) ?? undefined
  });

  const headers = Object.fromEntries(PASSED.flatMap((name) => (response.headers.has(name) ? [[name, response.headers.get(name)!]] : [])));

  // The event stream rides through open, as the console relays it.
  if ((headers["content-type"] ?? "").includes("text/event-stream")) return new Response(response.body, { status: response.status, headers });

  return new Response(response.status === 204 ? null : await response.arrayBuffer(), { status: response.status, headers });
});
