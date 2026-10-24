<script lang="ts">
  import { call, commonProblem } from "#lib/api.js";
  import { whenText } from "#lib/access-log.js";
  import { STRINGS } from "#lib/strings.js";

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

  let grants = $state<Grant[] | null>(null);
  let message = $state<string | null>(null);
  let open = $state<string | null>(null);
  let outcome = $state<"appropriate" | "inappropriate">("appropriate");
  let note = $state("");

  async function load() {
    const res = await call<{ grants: Grant[] }>(
      "GET",
      "/api/break-glass/reviews",
    );
    const problem = commonProblem(res.status);
    if (problem) {
      grants = null;
      message = STRINGS[problem];
    } else if (res.status === 200) {
      grants = res.body.grants;
    } else message = STRINGS.error;
  }
  $effect(() => {
    void load();
  });

  async function record(event: SubmitEvent) {
    event.preventDefault();
    if (!open) return;
    if (!note.trim()) {
      message = STRINGS.glassNoteNeeded;
      return;
    }
    const res = await call(
      "POST",
      `/api/break-glass/reviews/${open}`,
      { outcome, note: note.trim() },
    );
    const problem = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (problem) message = STRINGS[problem];
    else if (res.status === 200) message = STRINGS.glassReviewed;
    else if (res.status === 409) message = STRINGS.glassAlready;
    else if (res.status === 422 && body?.errors) message = STRINGS.glassOwn;
    else message = STRINGS.error;
    if (res.status === 200 || res.status === 409) {
      open = null;
      note = "";
      await load();
    }
  }
</script>

<svelte:head><title>{STRINGS.glassReviewTitle}</title></svelte:head>
<h1>{STRINGS.glassReviewTitle}</h1>
<p>{STRINGS.glassReviewIntro}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if grants !== null}
  {#if grants.length === 0}
    <p>{STRINGS.glassReviewNone}</p>
  {:else}
    <ul>
      {#each grants as g (g.id)}
        <li>
          {#if g.overdue}<strong class="overdue">{STRINGS.glassOverdue}</strong> ·{/if}
          {whenText(g.granted_at)} · {STRINGS.glassBy} <code>{g.subject}</code>
          · {STRINGS.glassFor} <code>{g.patient_id}</code>
          · {STRINGS.glassUntil} {whenText(g.expires_at)}
          <p>{STRINGS.glassWhy}: {g.reason}</p>
          <button type="button" onclick={() => (open = g.id)}>
            {STRINGS.glassReview}
          </button>
          {#if open === g.id}
            <form onsubmit={record}>
              <label for="g-outcome">{STRINGS.glassOutcome}</label>
              <select id="g-outcome" bind:value={outcome}>
                <option value="appropriate">
                  {STRINGS.glassOutcomes.appropriate}
                </option>
                <option value="inappropriate">
                  {STRINGS.glassOutcomes.inappropriate}
                </option>
              </select>
              <label for="g-note">{STRINGS.glassNote}</label>
              <input id="g-note" bind:value={note} autocomplete="off" />
              <button type="submit">{STRINGS.glassRecord}</button>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
