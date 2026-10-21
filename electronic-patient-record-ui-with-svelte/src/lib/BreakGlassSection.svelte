<script lang="ts">
  // Break-the-glass (SE-3): offered only when the API refused clinical content for want of a care
  // relationship and said break-the-glass could lift it. A reason and an acknowledgement are
  // required; nothing is sent without both. The grant lasts 4 hours and is reviewed afterwards.
  import { call, commonProblem } from "./api.js";
  import { STRINGS } from "./strings.js";

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
    if (!reason.trim() || !acknowledged) {
      message = STRINGS.glassNeeded;
      return;
    }
    const res = await call<{ expires_at: string }>(
      "POST",
      `/api/patients/${patientId}/break-glass`,
      { reason: reason.trim() },
    );
    const problem = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (problem) message = STRINGS[problem];
    else if (res.status === 200 || res.status === 201) {
      reason = "";
      acknowledged = false;
      await onopened(res.body.expires_at);
    } else if (res.status === 422 && body?.errors) message = body.errors.join(" ");
    else message = STRINGS.error;
  }
</script>

<section aria-labelledby="glass-h">
  <h2 id="glass-h">{STRINGS.glassTitle}</h2>
  <p>{STRINGS.glassIntro}</p>
  <form onsubmit={open}>
    <label for="glass-reason">{STRINGS.glassReason}</label>
    <input id="glass-reason" bind:value={reason} autocomplete="off" />
    <label>
      <input type="checkbox" bind:checked={acknowledged} />
      {STRINGS.glassAck}
    </label>
    <button type="submit">{STRINGS.glassOpen}</button>
  </form>
  {#if message}<p role="status">{message}</p>{/if}
</section>
