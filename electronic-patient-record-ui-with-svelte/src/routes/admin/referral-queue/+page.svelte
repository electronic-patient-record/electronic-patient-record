<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "#lib/api.js";
  import {
    clearBody,
    HOWS,
    queueHeadline,
    type How,
    type QueueEntry,
  } from "#lib/referral-queue.js";
  import { readable } from "#lib/feed-queue.js";
  import { refreshReferralQueueCount } from "#lib/feed-count.svelte.js";
  import { newIdempotencyKey } from "#lib/idempotency.js";
  import { TEXT } from "#lib/i18n.js";

  const busy = new Busy();
  let open = $state<QueueEntry[] | null>(null);
  let limit = $state(50);
  // The service says when the list was cut short (PR-1); an older one says nothing, so a full list counts as cut.
  let more = $state(false);
  let message = $state<string | null>(null);
  // One entry's message is shown at a time, fetched when asked for (a listing carries no text).
  let shown = $state<QueueEntry | null>(null);
  let siteZone = $state<string | undefined>(undefined);
  let how = $state<How>("dismissed");
  let referralId = $state("");
  let reason = $state("");
  let idempotencyKey = newIdempotencyKey();

  async function load() {
    const res = await call<{
      entries: QueueEntry[];
      limit: number;
      more?: boolean;
      site_time_zone?: string;
    }>("GET", "/api/referral-queue");
    const problem = commonProblem(res.status);
    if (problem) {
      open = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      open = res.body.entries;
      limit = res.body.limit;
      more = res.body.more ?? open.length >= limit;
      siteZone = res.body.site_time_zone;
      void refreshReferralQueueCount();
    } else message = TEXT.error;
  }
  $effect(() => {
    void load();
  });

  async function toggle(e: QueueEntry) {
    await busy.run(() => toggleNow(e));
  }

  async function toggleNow(e: QueueEntry) {
    if (shown?.id === e.id) {
      shown = null;
      return;
    }
    const res = await call<QueueEntry>("GET", `/api/referral-queue/${e.id}`);
    const problem = commonProblem(res.status);
    if (problem) message = TEXT[problem];
    else if (res.status === 200) {
      shown = res.body;
      how = "dismissed";
      referralId = "";
      reason = "";
      idempotencyKey = newIdempotencyKey();
      message = null;
    } else message = TEXT.error;
  }

  async function clear(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => clearNow());
  }

  async function clearNow() {
    if (!shown) return;
    const body = clearBody(how, referralId, reason);
    if (!body) {
      message = TEXT.rqNeeded;
      return;
    }
    const res = await call(
      "POST",
      `/api/referral-queue/${shown.id}/clear`,
      body,
      undefined,
      { idempotencyKey },
    );
    const problem = commonProblem(res.status);
    if (problem) message = TEXT[problem];
    else if (res.status === 200) message = TEXT.rqCleared;
    else if (res.status === 409) message = TEXT.rqAlready;
    else message = TEXT.error;
    if (!problem && (res.status === 200 || res.status === 409)) {
      shown = null;
      await load();
    }
  }
</script>

<svelte:head><title>{TEXT.rqTitle}</title></svelte:head>
<h1>{TEXT.rqTitle}</h1>
<p>{TEXT.rqIntro}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if open !== null}
  {#if open.length === 0}
    <p>{TEXT.rqNone}</p>
  {:else}
    <p>{TEXT.rqCount.replace("{n}", String(open.length))}</p>
    {#if more}
      <p>{TEXT.rqMore.replace("{n}", String(limit))}</p>
    {/if}
    <ul>
      {#each open as e (e.id)}
        <li>
          <strong>{TEXT.rqKinds[e.kind]}</strong> · {queueHeadline(e, siteZone)}
          <ul>
            {#each e.reasons as why (why)}<li>{why}</li>{/each}
          </ul>
          <button
            disabled={busy.on}
            type="button"
            onclick={() => void toggle(e)}
          >
            {shown?.id === e.id ? TEXT.rqHide : TEXT.rqShow}
          </button>
          {#if shown?.id === e.id}
            {#if shown.note}
              <h2 id="rq-note-h">{TEXT.rqNote}</h2>
              <p aria-labelledby="rq-note-h">{shown.note}</p>
            {/if}
            <h2 id="rq-msg-h">{TEXT.rqMessage}</h2>
            <pre aria-labelledby="rq-msg-h">{readable(
                shown.message ?? "",
              )}</pre>
            <form onsubmit={clear}>
              <label for="rq-how">{TEXT.rqHow}</label>
              <select id="rq-how" bind:value={how}>
                {#each HOWS as h (h)}
                  <option value={h}>{TEXT.rqHows[h]}</option>
                {/each}
              </select>
              {#if how === "matched"}
                <label for="rq-ref">{TEXT.rqReferralId}</label>
                <input id="rq-ref" bind:value={referralId} autocomplete="off" />
              {/if}
              <label for="rq-reason">{TEXT.rqReason}</label>
              <input id="rq-reason" bind:value={reason} autocomplete="off" />
              <button disabled={busy.on} type="submit">{TEXT.rqClear}</button>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
