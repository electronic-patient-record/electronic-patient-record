<script lang="ts">
  // Watch lists (WL-6) on a patient's page: who has this patient on their list, since when, why, and
  // until when; adding them to one's own list with a reason; ending an entry with a reason. Only the
  // owner may end an entry, which the service decides: anyone else is told so, not hidden the button.
  // A 403 or a failed load says so, never "none". Text from the record is shown as text.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import { siteWhenText } from "./access-log.js";
  import {
    buildWatchEntry,
    loadWatchList,
    type WatchEntry,
    type WatchLoad,
  } from "./watch-list.js";
  import { TEXT } from "./i18n.js";

  let {
    patientId,
    siteZone = "UTC",
    refresh = null,
  }: { patientId: string; siteZone?: string; refresh?: unknown } = $props();

  const busy = new Busy();
  let res = $state<WatchLoad | null>(null);
  let reason = $state("");
  let until = $state("");
  let ending = $state<string | null>(null);
  let endReason = $state("");
  let message = $state<string | null>(null);

  let ticket = 0;
  async function load() {
    const mine = ++ticket;
    const forId = patientId;
    const loaded = await loadWatchList(forId);
    if (mine === ticket && forId === patientId) res = loaded;
  }
  $effect(() => {
    void patientId;
    void refresh;
    res = null;
    ending = null;
    void load();
  });

  function problemOf(status: number, body: unknown): string | null {
    const common = commonProblem(status);
    const errors = (body as { errors?: string[] } | null)?.errors;
    if (common) return TEXT[common];
    if (status === 422 && errors) return errors.join(" ");
    return null;
  }

  async function add(event: SubmitEvent) {
    event.preventDefault();
    const built = buildWatchEntry(reason, until);
    if (built.problem) {
      message = TEXT.watchNeedReason;
      return;
    }
    await busy.run(async () => {
      message = null;
      const r = await call(
        "POST",
        `/api/patients/${patientId}/watch-list`,
        built.body,
      );
      if (r.status === 201) {
        message = TEXT.watchAdded;
        reason = until = "";
        await load();
      } else if (r.status === 409) message = TEXT.watchAlready;
      else message = problemOf(r.status, r.body) ?? TEXT.error;
    });
  }

  async function end(e: WatchEntry, event: SubmitEvent) {
    event.preventDefault();
    if (!endReason.trim()) {
      message = TEXT.watchNeedReason;
      return;
    }
    await busy.run(async () => {
      message = null;
      const r = await call(
        "PUT",
        `/api/patients/${patientId}/watch-list/${e.id}`,
        { expected_version: e.version, end_reason: endReason.trim() },
      );
      if (r.status === 200) {
        message = TEXT.watchEnded;
        ending = null;
        endReason = "";
        await load();
      } else if (r.status === 404) message = TEXT.watchNotOwner;
      else if (r.status === 409) {
        message = TEXT.referralChanged;
        await load();
      } else message = problemOf(r.status, r.body) ?? TEXT.error;
    });
  }
</script>

<section aria-labelledby="watch-h">
  <h2 id="watch-h">{TEXT.watchTitle}</h2>
  <p role="note">{TEXT.watchNote}</p>
  {#if res === null}
    <p>{TEXT.loading}</p>
  {:else if res.status === 403}
    <p>{TEXT.watchNotShown}</p>
  {:else if res.status !== 200 || !res.entries}
    <p>{TEXT.watchCouldNot}</p>
  {:else}
    {#if res.entries.length === 0}
      <p>{TEXT.watchNone}</p>
    {:else}
      <ul>
        {#each res.entries as e (e.id)}
          <li>
            <strong>{TEXT.watchStates[e.state]}</strong>
            · {TEXT.watchEntry
              .replace("{since}", siteWhenText(e.since, siteZone))
              .replace("{owner}", e.owner)}
            {#if e.until}· {TEXT.watchUntil.replace("{until}", e.until)}{/if}
            <p>{TEXT.watchReason}: {e.reason}</p>
            {#if e.end_reason}
              <p>{TEXT.watchEndReason}: {e.end_reason}</p>
            {/if}
            {#if e.state === "active" || e.state === "lapsed"}
              {#if ending === e.id}
                <form onsubmit={(ev) => end(e, ev)} novalidate>
                  <label for="watch-end-{e.id}">{TEXT.watchEndLabel}</label>
                  <input
                    id="watch-end-{e.id}"
                    bind:value={endReason}
                    maxlength="500"
                    autocomplete="off"
                  />
                  <button disabled={busy.on} type="submit"
                    >{TEXT.watchEndSend}</button
                  >
                </form>
              {:else}
                <button
                  type="button"
                  disabled={busy.on}
                  onclick={() => {
                    ending = e.id;
                    endReason = "";
                    message = null;
                  }}>{TEXT.watchEnd}</button
                >
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    <form onsubmit={add} novalidate>
      <h3>{TEXT.watchAddTitle}</h3>
      <label for="watch-reason">{TEXT.watchAddReason}</label>
      <input
        id="watch-reason"
        bind:value={reason}
        maxlength="500"
        autocomplete="off"
      />
      <label for="watch-until">{TEXT.watchAddUntil}</label>
      <input id="watch-until" type="date" bind:value={until} />
      <button disabled={busy.on} type="submit">{TEXT.watchAdd}</button>
    </form>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
