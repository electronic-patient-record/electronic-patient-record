<script lang="ts">
  // Report that a patient has died, and act on a report (BV-1, H-046). Any member of staff may report;
  // who may read the reasons and take a step is the service's to decide (OQ-BV2), so a role that may
  // not sees the report form and a sentence saying the rest is not shown. Who acts is the token.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import {
    SOURCES,
    assertionLine,
    emptyReport,
    loadAssertions,
    reportBody,
    statusText,
    stepBody,
    stepsFor,
    type Status,
    type StoredAssertion,
  } from "./death.js";
  import { STRINGS } from "./strings.js";

  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
    onchange,
  }: {
    patientId: string;
    siteZone?: string;
    onchange: () => void | Promise<void>;
  } = $props();

  let status = $state<number | null>(null);
  let rows = $state<StoredAssertion[]>([]);
  let message = $state<string | null>(null);
  let form = $state(emptyReport());
  let reporting = $state(false);
  // The one step being filled in, on one assertion.
  let open = $state<{ id: string; to: Status } | null>(null);
  let reason = $state("");

  // A late answer for the patient the page has moved away from is dropped.
  let ticket = 0;
  async function load() {
    const mine = ++ticket;
    const forId = patientId;
    const res = await loadAssertions(forId);
    if (mine !== ticket || forId !== patientId) return;
    status = res.status;
    rows = res.assertions;
  }
  $effect(() => {
    void patientId;
    status = null;
    rows = [];
    open = null;
    reporting = false;
    form = emptyReport();
    message = null;
    void load();
  });

  function failed(res: { status: number; body?: { errors?: string[] } }) {
    const common = commonProblem(res.status);
    if (common) return STRINGS[common];
    if (res.status === 409) return STRINGS.deathConflict;
    if (res.status === 422 && res.body?.errors)
      return res.body.errors.join(" ");
    return STRINGS.error;
  }

  async function report(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(async () => {
      message = null;
      const built = reportBody(form);
      if (!built.ok) {
        message = built.problem;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "POST",
        `/api/patients/${patientId}/death/assertions`,
        built.body,
      );
      if (res.status === 201) {
        message = STRINGS.deathReported;
        reporting = false;
        form = emptyReport();
        await Promise.all([load(), onchange()]);
      } else message = failed(res);
    });
  }

  async function step(event: SubmitEvent, a: StoredAssertion) {
    event.preventDefault();
    if (!open) return;
    const to = open.to;
    await busy.run(async () => {
      message = null;
      const built = stepBody(to, a.version, reason);
      if (!built.ok) {
        message = built.problem;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "PUT",
        `/api/patients/${patientId}/death/assertions/${a.id}`,
        built.body,
      );
      if (res.status === 200) {
        message = STRINGS.deathDone;
        open = null;
        await Promise.all([load(), onchange()]);
      } else message = failed(res);
    });
  }
</script>

<section aria-labelledby="death-h">
  <h2 id="death-h">{STRINGS.deathTitle}</h2>
  {#if status === null}
    <p>{STRINGS.loading}</p>
  {:else if status === 403}
    <p>{STRINGS.deathNotShown}</p>
  {:else if status !== 200}
    <p>{STRINGS.deathCouldNot}</p>
  {:else if rows.length === 0}
    <p>{STRINGS.deathNone}</p>
  {:else}
    <ul>
      {#each rows as a (a.id)}
        <li>
          <strong>{assertionLine(a, siteZone)}</strong>
          <p>{STRINGS.deathReasonOf} {a.assertion.reason}</p>
          {#if open?.id === a.id}
            <form onsubmit={(e) => step(e, a)}>
              <p>
                {STRINGS.deathStepTo.replace("{status}", statusText(open.to))}
              </p>
              <label for="death-r-{a.id}">{STRINGS.deathReason}</label>
              <input
                id="death-r-{a.id}"
                bind:value={reason}
                autocomplete="off"
              />
              <button type="submit" disabled={busy.on}
                >{STRINGS.deathConfirm}</button
              >
              <button type="button" onclick={() => (open = null)}>
                {STRINGS.deathCancel}
              </button>
            </form>
          {:else}
            {#each stepsFor(a.assertion.status) as to (to)}
              <button
                type="button"
                onclick={() => {
                  open = { id: a.id, to };
                  reason = "";
                  message = null;
                }}
              >
                {statusText(to)}
              </button>
            {/each}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if reporting}
    <form onsubmit={report}>
      <label for="death-source">{STRINGS.deathSource}</label>
      <select id="death-source" bind:value={form.source}>
        <option value=""></option>
        {#each SOURCES as s (s)}
          <option value={s}>{STRINGS.deathSources[s]}</option>
        {/each}
      </select>
      <label for="death-date">{STRINGS.deathDate}</label>
      <input id="death-date" bind:value={form.date} autocomplete="off" />
      <label for="death-no-date">{STRINGS.deathNoDate}</label>
      <input
        id="death-no-date"
        bind:value={form.noDateReason}
        autocomplete="off"
      />
      <label for="death-reason">{STRINGS.deathReason}</label>
      <input id="death-reason" bind:value={form.reason} autocomplete="off" />
      <button type="submit" disabled={busy.on}>{STRINGS.deathReportSend}</button
      >
      <button type="button" onclick={() => (reporting = false)}>
        {STRINGS.deathCancel}
      </button>
    </form>
  {:else}
    <button type="button" onclick={() => (reporting = true)}>
      {STRINGS.deathReport}
    </button>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
