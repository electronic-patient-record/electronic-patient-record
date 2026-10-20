# electronic-patient-record (UI)

SvelteKit shell. **Not for clinical use.**

```sh
pnpm install
pnpm dev                 # http://localhost:5173, proxies to EPR_API_URL (default http://localhost:5150)
pnpm run check && pnpm exec vitest run && PW_PORT=4183 CI=true pnpm exec playwright test
```
