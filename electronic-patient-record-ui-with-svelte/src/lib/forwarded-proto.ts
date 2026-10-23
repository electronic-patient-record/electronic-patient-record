// The one place that reads EPR_TRUST_FORWARDED_PROTO, so every route decides "is this HTTPS?" alike.
import * as env from "$app/env/private";
import { trustedForwardedProto } from "./security-headers.js";

export function forwardedProto(request: Request): string | null {
  return trustedForwardedProto(request.headers, env.EPR_TRUST_FORWARDED_PROTO);
}
