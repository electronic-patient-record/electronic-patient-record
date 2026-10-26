<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // The vital signs list and the form to record one (RC-6, first UI slice). Nothing here says a value
  // is normal unless the API computed it from a range the sender gave; a missing value says why; an
  // empty box is never sent as 0. Correcting and retracting are a later slice.
  import { call, commonProblem } from "./api.js";
  import {
    VITALS,
    formToRequest,
    loadObservations,
    nowInZone,
    rangeText,
    unitLabel,
    valueText,
    assessmentText,
    vitalName,
    type AbsentReason,
    type StoredObservation,
    type VitalKey,
  } from "./vitals.js";
  import { siteWhenText } from "./access-log.js";
  import { STRINGS } from "./strings.js";

  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
    refresh = null,
  }: { patientId: string; siteZone?: string; refresh?: unknown } = $props();

  let status = $state<number | null>(null);
  let rows = $state<StoredObservation[]>([]);
  let message = $state<string | null>(null);

  let vital = $state<VitalKey>("heartRate");
  let value = $state("");
  let noValue = $state(false);
  let absentReason = $state<AbsentReason | "">("");
  let low = $state("");
  let high = $state("");
  let measuredAt = $state("");
  let why = $state("");

  const unit = $derived(unitLabel(VITALS.find((v) => v.key === vital)?.unit ?? ""));
  const standing = $derived(rows.filter((o) => o.status !== "entered-in-error"));
  const mistaken = $derived(rows.filter((o) => o.status === "entered-in-error"));

  // A late answer for the patient the page has moved away from is dropped.
  let ticket = 0;
  async function load() {
    const mine = ++ticket;
    const forId = patientId;
    const res = await loadObservations(forId);
    if (mine !== ticket || forId !== patientId) return;
    status = res.status;
    rows = res.observations ?? [];
  }
  $effect(() => {
    void patientId;
    void refresh;
    status = null;
    rows = [];
    measuredAt = nowInZone(new Date(), siteZone);
    void load();
  });

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(recordNow);
  }

  async function recordNow() {
    message = null;
    const built = formToRequest({ vital, value, noValue, absentReason, low, high, measuredAt, siteZone });
    if (!built.ok) {
      message = built.problem;
      return;
    }
    const res = await call<{ errors?: string[] }>("POST", `/api/patients/${patientId}/observations`, {
      observation: built.record,
      reason: why.trim() || undefined,
    });
    const common = commonProblem(res.status);
    if (common) message = STRINGS[common];
    else if (res.status === 201) {
      message = STRINGS.vitalsSaved;
      value = low = high = why = "";
      noValue = false;
      absentReason = "";
      measuredAt = nowInZone(new Date(), siteZone);
      await load();
    } else if (res.status === 409) {
      message = STRINGS.vitalConflict;
    } else if (res.status === 422 && res.body?.errors) message = res.body.errors.join(" ");
    else message = STRINGS.error;
  }
</script>

<section aria-labelledby="vitals-h">
  <h2 id="vitals-h">{STRINGS.vitalsTitle}</h2>
  {#if status === null}
    <p>{STRINGS.loading}</p>
  {:else if status === 403}
    <p>{STRINGS.vitalsNotShown}</p>
  {:else if status !== 200}
    <p>{STRINGS.vitalsCouldNot}</p>
  {:else}
    {#if standing.length === 0}
      <p>{STRINGS.vitalsNone}</p>
    {:else}
      <ul>
        {#each standing as o (o.id)}
          <li>
            <strong>{vitalName(o.code)}</strong>
            · {valueText(o)}
            · {assessmentText(o)}
            {#if rangeText(o.range)}· {STRINGS.vitalRange} {rangeText(o.range)}{/if}
            · <time datetime={o.effective_at}>{siteWhenText(o.effective_at, siteZone)}</time>
            · {STRINGS.vitalBy} <code>{o.author}</code>
            {#if o.version > 1}· {STRINGS.vitalVersion.replace("{n}", String(o.version))}{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if mistaken.length}
      <details>
        <summary>{STRINGS.vitalInError.replace("{n}", String(mistaken.length))}</summary>
        <ul>
          {#each mistaken as o (o.id)}
            <li>
              <s>{vitalName(o.code)} · {valueText(o)}</s>
              · <time datetime={o.effective_at}>{siteWhenText(o.effective_at, siteZone)}</time>
            </li>
          {/each}
        </ul>
      </details>
    {/if}

    <h3>{STRINGS.vitalsRecordTitle}</h3>
    <form onsubmit={record}>
      <label for="vs-vital">{STRINGS.vitalLabel}</label>
      <select id="vs-vital" bind:value={vital}>
        {#each VITALS as v (v.key)}
          <option value={v.key}>{STRINGS.vitalNames[v.key]}</option>
        {/each}
      </select>
      <label for="vs-value">{STRINGS.vitalValueLabel} ({unit})</label>
      <input
        id="vs-value"
        bind:value
        inputmode="decimal"
        autocomplete="off"
        disabled={noValue}
        required={!noValue}
      />
      <label>
        <input type="checkbox" bind:checked={noValue} />
        {STRINGS.vitalNoValueTick}
      </label>
      {#if noValue}
        <label for="vs-absent">{STRINGS.vitalReasonLabel}</label>
        <select id="vs-absent" bind:value={absentReason} required>
          <option value=""></option>
          {#each Object.entries(STRINGS.vitalAbsent) as [k, v] (k)}
            <option value={k}>{v}</option>
          {/each}
        </select>
      {/if}
      <label for="vs-at">{STRINGS.vitalMeasuredAt.replace("{zone}", siteZone)}</label>
      <input id="vs-at" type="datetime-local" bind:value={measuredAt} required />
      <label for="vs-low">{STRINGS.vitalRangeLow}</label>
      <input id="vs-low" bind:value={low} inputmode="decimal" autocomplete="off" />
      <label for="vs-high">{STRINGS.vitalRangeHigh}</label>
      <input id="vs-high" bind:value={high} inputmode="decimal" autocomplete="off" />
      <label for="vs-why">{STRINGS.vitalWhy}</label>
      <input id="vs-why" bind:value={why} autocomplete="off" />
      <button disabled={busy.on} type="submit">{STRINGS.vitalSubmit}</button>
    </form>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
