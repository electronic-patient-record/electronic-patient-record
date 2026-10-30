import { defineEnvVars } from "@sveltejs/kit/env";

export const variables = defineEnvVars({
  // Base URL of the service, read server-side by the BFF proxy.
  EPR_API_URL: { schema: (input) => input ?? "" },
  // Sign-in (all optional; unset EPR_OIDC_ISSUER leaves sign-in off). The issuer is the
  // Keycloak realm URL; the public URL is where the browser reaches this UI.
  EPR_OIDC_ISSUER: { schema: (input) => input ?? "" },
  EPR_OIDC_CLIENT_ID: { schema: (input) => input ?? "" },
  EPR_PUBLIC_URL: { schema: (input) => input ?? "" },
  // "1" or "true" when this UI sits behind a proxy that sets X-Forwarded-Proto itself; otherwise the
  // header is ignored, because a client could send it to get Secure cookies or HSTS decided for it.
  EPR_TRUST_FORWARDED_PROTO: { schema: (input) => input ?? "" },
  // Share sign-ins between UI replicas (OS-8). Both or neither: sessions are encrypted with the key
  // (32 bytes, base64) before they are written to Redis. Unset = sessions live in this process.
  EPR_SESSION_REDIS_URL: { schema: (input) => input ?? "" },
  EPR_SESSION_ENCRYPTION_KEY: { schema: (input) => input ?? "" },
  EPR_SESSION_IDLE_SECONDS: { schema: (input) => input ?? "" },
  EPR_SESSION_ABSOLUTE_SECONDS: { schema: (input) => input ?? "" },
});
