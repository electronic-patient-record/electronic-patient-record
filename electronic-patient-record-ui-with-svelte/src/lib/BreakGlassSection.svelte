<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // Break-the-glass (SE-3): offered only when the API refused clinical content for want of a care
  // relationship and said break-the-glass could lift it. A reason, an answer to each question about
  // the emergency (RF-6, PI-10) and an acknowledgement are required; nothing is sent without all of
  // them. "Not known" is an answer. Each answer is the opener's word: nothing here works one out.
  // The grant lasts 4 hours and is reviewed afterwards.
  import { call, commonProblem } from "./api.js";
  import { TEXT } from "./i18n.js";

  type Threat = "life" | "limb" | "neither" | "not-known";
  type TimeToHarm = "minutes" | "hours" | "days" | "not-known";
  type CanBeAsked = "yes" | "no" | "not-known";
  const THREATS: Threat[] = ["life", "limb", "neither", "not-known"];
  const TIMES: TimeToHarm[] = ["minutes", "hours", "days", "not-known"];
  const ASKS: CanBeAsked[] = ["yes", "no", "not-known"];

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
  // Nothing is chosen for the opener: an answer left unchosen blocks the form.
  let threat = $state<Threat | null>(null);
  let timeToHarm = $state<TimeToHarm | null>(null);
  let canBeAsked = $state<CanBeAsked | null>(null);
  let said = $state("");
  let acknowledged = $state(false);
  let message = $state<string | null>(null);

  async function open(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => openNow());
  }

  async function openNow() {
    const answered =
      threat !== null &&
      timeToHarm !== null &&
      canBeAsked !== null &&
      (canBeAsked !== "yes" || said.trim() !== "");
    if (!reason.trim() || !acknowledged || !answered) {
      message = TEXT.glassNeeded;
      return;
    }
    const res = await call<{ expires_at: string }>(
      "POST",
      `/api/patients/${patientId}/break-glass`,
      {
        reason: reason.trim(),
        emergency: {
          threat,
          time_to_harm: timeToHarm,
          can_be_asked: canBeAsked,
          asked_and_said: canBeAsked === "yes" ? said.trim() : null,
        },
      },
    );
    const problem = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (problem) message = TEXT[problem];
    else if (res.status === 200 || res.status === 201) {
      reason = "";
      said = "";
      threat = timeToHarm = canBeAsked = null;
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
    <fieldset>
      <legend>{TEXT.glassThreat}</legend>
      {#each THREATS as t (t)}
        <label>
          <input
            type="radio"
            name="glass-threat"
            value={t}
            bind:group={threat}
          />
          {TEXT.glassThreats[t]}
        </label>
      {/each}
    </fieldset>
    <fieldset>
      <legend>{TEXT.glassTime}</legend>
      {#each TIMES as t (t)}
        <label>
          <input
            type="radio"
            name="glass-time"
            value={t}
            bind:group={timeToHarm}
          />
          {TEXT.glassTimes[t]}
        </label>
      {/each}
    </fieldset>
    <fieldset>
      <legend>{TEXT.glassAsk}</legend>
      {#each ASKS as a (a)}
        <label>
          <input
            type="radio"
            name="glass-ask"
            value={a}
            bind:group={canBeAsked}
          />
          {TEXT.glassAsks[a]}
        </label>
      {/each}
    </fieldset>
    {#if canBeAsked === "yes"}
      <label for="glass-said">{TEXT.glassSaid}</label>
      <input id="glass-said" bind:value={said} autocomplete="off" />
    {/if}
    <label>
      <input type="checkbox" bind:checked={acknowledged} />
      {TEXT.glassAck}
    </label>
    <button disabled={busy.on} type="submit">{TEXT.glassOpen}</button>
  </form>
  {#if message}<p role="status">{message}</p>{/if}
</section>
