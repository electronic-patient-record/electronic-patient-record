import { defineEnvVars } from "@sveltejs/kit/env";

export const variables = defineEnvVars({
  // Base URL of the service, read server-side by the BFF proxy.
  EPR_API_URL: { schema: (input) => input ?? "" },
});
