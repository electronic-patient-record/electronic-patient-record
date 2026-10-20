import * as env from "$app/env/private";
import { error, type RequestHandler } from "@sveltejs/kit";
import { proxyTarget } from "#lib/proxy.js";

// The backend-for-frontend proxy. The browser never talks to the API directly.
const forward: RequestHandler = async ({ params, url, request }) => {
  const base = env.EPR_API_URL || "http://localhost:5150";
  const target = proxyTarget(base, params.path ?? "", url.search);
  if (!target) error(404, "not found");
  const upstream = await fetch(target, {
    method: request.method,
    headers: { "content-type": request.headers.get("content-type") ?? "application/json" },
    body: request.method === "GET" ? undefined : await request.text(),
  });
  return new Response(upstream.body, {
    status: upstream.status,
    headers: { "content-type": upstream.headers.get("content-type") ?? "application/json" },
  });
};

export const GET = forward;
export const POST = forward;
