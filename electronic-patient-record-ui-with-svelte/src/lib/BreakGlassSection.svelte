<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // Break-the-glass (SE-3): offered only when the API refused clinical content for want of a care
  // relationship and said break-the-glass could lift it. A reason and an acknowledgement are
  // required; nothing is sent without both. The grant lasts 4 hours and is reviewed afterwards.
  import { call, commonProblem } from "./api.js";
  import { TEXT } from "./i18n.js";

  const busy = new Busy();
  let {
    patientId,
    onopened,
  }: {
    patientId: string;
    // Called with the end time after a grant is opened. The page reads again under it and keeps
    // saying so, because this section goes away once the content is shown.
    onopened: (expiresAt: string) => void | Promise<void>;
  } = $props();

  let reason = $state("");
  let acknowledged = $state(false);
  let message = $state<string | null>(null);

  async function open(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => openNow());
  }

  async function openNow() {
    if (!reason.trim() || !acknowledged) {
      message = TEXT.glassNeeded;
      return;
    }
    const res = await call<{ expires_at: string }>(
      "POST",
      `/api/patients/${patientId}/break-glass`,
      { reason: reason.trim() },
    );
    const problem = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (problem) message = TEXT[problem];
    else if (res.status === 200 || res.status === 201) {
      reason = "";
      acknowledged = false;
      await onopened(res.body.expires_at);
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = TEXT.error;
  }
</script>

<section aria-labelledby="glass-h">
  <h2 id="glass-h">{TEXT.glassTitle}</h2>
  <p>{TEXT.glassIntro}</p>
  <form onsubmit={open}>
    <label for="glass-reason">{TEXT.glassReason}</label>
    <input id="glass-reason" bind:value={reason} autocomplete="off" />
    <label>
      <input type="checkbox" bind:checked={acknowledged} />
      {TEXT.glassAck}
    </label>
    <button disabled={busy.on} type="submit">{TEXT.glassOpen}</button>
  </form>
  {#if message}<p role="status">{message}</p>{/if}
</section>
