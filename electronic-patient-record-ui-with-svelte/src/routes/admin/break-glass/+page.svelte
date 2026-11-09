<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "#lib/api.js";
  import { siteWhenText } from "#lib/access-log.js";
  import { TEXT } from "#lib/i18n.js";

  type Grant = {
    id: string;
    patient_id: string;
    subject: string;
    reason: string;
    granted_at: string;
    expires_at: string;
    /** Unreviewed for longer than the service's limit. Absent from an older API: not overdue. */
    overdue?: boolean;
  };

  const busy = new Busy();
  let grants = $state<Grant[] | null>(null);
  let limit = $state(50);
  // The service says when the list was cut short; an older API says nothing, so a full list counts as cut.
  let more = $state(false);
  let siteZone = $state<string | undefined>(undefined);
  let message = $state<string | null>(null);
  let open = $state<string | null>(null);
  let outcome = $state<"appropriate" | "inappropriate">("appropriate");
  let note = $state("");

  async function load() {
    const res = await call<{
      grants: Grant[];
      limit?: number;
      more?: boolean;
      site_time_zone?: string;
    }>("GET", "/api/break-glass/reviews");
    const problem = commonProblem(res.status);
    if (problem) {
      grants = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      grants = res.body.grants;
      limit = res.body.limit ?? 50;
      more = res.body.more ?? grants.length >= limit;
      siteZone = res.body.site_time_zone;
    } else message = TEXT.error;
  }
  $effect(() => {
    void load();
  });

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => recordNow());
  }

  async function recordNow() {
    if (!open) return;
    if (!note.trim()) {
      message = TEXT.glassNoteNeeded;
      return;
    }
    const res = await call("POST", `/api/break-glass/reviews/${open}`, {
      outcome,
      note: note.trim(),
    });
    const problem = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (problem) message = TEXT[problem];
    else if (res.status === 200) message = TEXT.glassReviewed;
    else if (res.status === 409) message = TEXT.glassAlready;
    else if (res.status === 422 && body?.errors) message = TEXT.glassOwn;
    else message = TEXT.error;
    if (res.status === 200 || res.status === 409) {
      open = null;
      note = "";
      await load();
    }
  }
</script>

<svelte:head><title>{TEXT.glassReviewTitle}</title></svelte:head>
<h1>{TEXT.glassReviewTitle}</h1>
<p>{TEXT.glassReviewIntro}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if grants !== null}
  {#if grants.length === 0}
    <p>{TEXT.glassReviewNone}</p>
  {:else}
    {#if more}<p role="status">
        {TEXT.glassReviewMore.replace("{n}", String(limit))}
      </p>{/if}
    <ul>
      {#each grants as g (g.id)}
        <li>
          {#if g.overdue}<strong class="overdue">{TEXT.glassOverdue}</strong> ·{/if}
          {siteWhenText(g.granted_at, siteZone)} · {TEXT.glassBy}
          <code>{g.subject}</code>
          · {TEXT.glassFor} <code>{g.patient_id}</code>
          · {TEXT.glassUntil}
          {siteWhenText(g.expires_at, siteZone)}
          <p>{TEXT.glassWhy}: {g.reason}</p>
          <button type="button" onclick={() => (open = g.id)}>
            {TEXT.glassReview}
          </button>
          {#if open === g.id}
            <form onsubmit={record}>
              <label for="g-outcome">{TEXT.glassOutcome}</label>
              <select id="g-outcome" bind:value={outcome}>
                <option value="appropriate">
                  {TEXT.glassOutcomes.appropriate}
                </option>
                <option value="inappropriate">
                  {TEXT.glassOutcomes.inappropriate}
                </option>
              </select>
              <label for="g-note">{TEXT.glassNote}</label>
              <input id="g-note" bind:value={note} autocomplete="off" />
              <button disabled={busy.on} type="submit"
                >{TEXT.glassRecord}</button
              >
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
