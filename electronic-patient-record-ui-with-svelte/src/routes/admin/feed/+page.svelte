<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "#lib/api.js";
  import {
    headline,
    OUTCOMES,
    readable,
    resolveBody,
    type Outcome,
    type Refusal,
  } from "#lib/feed-queue.js";
  import { refreshFeedCount } from "#lib/feed-count.svelte.js";
  import { STRINGS } from "#lib/strings.js";

  const busy = new Busy();
  let open = $state<Refusal[] | null>(null);
  let limit = $state(50);
  let message = $state<string | null>(null);
  // One entry's message is shown at a time, fetched when asked for (a listing carries no text).
  let shown = $state<Refusal | null>(null);
  let siteZone = $state<string | undefined>(undefined);
  let outcome = $state<Outcome>("handled");
  let reason = $state("");

  async function load() {
    const res = await call<{
      refusals: Refusal[];
      limit: number;
      site_time_zone?: string;
    }>("GET", "/api/feed/refusals");
    const problem = commonProblem(res.status);
    if (problem) {
      open = null;
      message = STRINGS[problem];
    } else if (res.status === 200) {
      open = res.body.refusals;
      limit = res.body.limit;
      siteZone = res.body.site_time_zone;
      void refreshFeedCount();
    } else message = STRINGS.error;
  }
  $effect(() => {
    void load();
  });

  async function toggle(r: Refusal) {
    await busy.run(() => toggleNow(r));
  }

  async function toggleNow(r: Refusal) {
    if (shown?.id === r.id) {
      shown = null;
      return;
    }
    const res = await call<Refusal>("GET", `/api/feed/refusals/${r.id}`);
    const problem = commonProblem(res.status);
    if (problem) message = STRINGS[problem];
    else if (res.status === 200) {
      shown = res.body;
      outcome = "handled";
      reason = "";
      message = null;
    } else message = STRINGS.error;
  }

  async function close(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => closeNow());
  }

  async function closeNow() {
    if (!shown) return;
    const body = resolveBody(outcome, reason);
    if (!body) {
      message = STRINGS.feedReasonNeeded;
      return;
    }
    const res = await call(
      "POST",
      `/api/feed/refusals/${shown.id}/resolve`,
      body,
    );
    const problem = commonProblem(res.status);
    if (problem) message = STRINGS[problem];
    else if (res.status === 200) message = STRINGS.feedClosed;
    else if (res.status === 409) message = STRINGS.feedAlready;
    else message = STRINGS.error;
    if (!problem && (res.status === 200 || res.status === 409)) {
      shown = null;
      await load();
    }
  }
</script>

<svelte:head><title>{STRINGS.feedTitle}</title></svelte:head>
<h1>{STRINGS.feedTitle}</h1>
<p>{STRINGS.feedIntro}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if open !== null}
  {#if open.length === 0}
    <p>{STRINGS.feedNone}</p>
  {:else}
    <p>{STRINGS.feedCount.replace("{n}", String(open.length))}</p>
    {#if open.length >= limit}
      <p>{STRINGS.feedMore.replace("{n}", String(limit))}</p>
    {/if}
    <ul>
      {#each open as r (r.id)}
        <li>
          <strong>{STRINGS.feedKinds[r.kind]}</strong> · {headline(r, siteZone)}
          <ul>
            {#each r.reasons as why (why)}<li>{why}</li>{/each}
          </ul>
          <button
            disabled={busy.on}
            type="button"
            onclick={() => void toggle(r)}
          >
            {shown?.id === r.id ? STRINGS.feedHide : STRINGS.feedShow}
          </button>
          {#if shown?.id === r.id}
            <h2 id="msg-h">{STRINGS.feedMessage}</h2>
            <pre aria-labelledby="msg-h">{readable(shown.message ?? "")}</pre>
            <form onsubmit={close}>
              <label for="f-outcome">{STRINGS.feedOutcome}</label>
              <select id="f-outcome" bind:value={outcome}>
                {#each OUTCOMES as o (o)}
                  <option value={o}>{STRINGS.feedOutcomes[o]}</option>
                {/each}
              </select>
              <label for="f-reason">{STRINGS.feedReason}</label>
              <input id="f-reason" bind:value={reason} autocomplete="off" />
              <button disabled={busy.on} type="submit"
                >{STRINGS.feedClose}</button
              >
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
