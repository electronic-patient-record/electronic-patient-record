<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // The vital signs list and the form to record one (RC-6, first UI slice). Nothing here says a value
  // is normal unless the API computed it from a range the sender gave; a missing value says why; an
  // empty box is never sent as 0. Marking an entry entered in error and correcting a value are here: a correction is a new version with a required reason, and the old version is kept.
  import { call, commonProblem } from "./api.js";
  import {
    VITALS,
    enteredInErrorRecord,
    formToRequest,
    loadHistory,
    type ObservationVersion,
    loadObservations,
    noticeText,
    type Notice,
    nowInZone,
    rangeText,
    trends,
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
  let notices = $state<Record<string, Notice[]>>({});
  let message = $state<string | null>(null);

  let vital = $state<VitalKey>("heartRate");
  let value = $state("");
  let noValue = $state(false);
  let absentReason = $state<AbsentReason | "">("");
  let low = $state("");
  let high = $state("");
  let measuredAt = $state("");
  let why = $state("");

  // The entry being corrected: the form is loaded from it and submits a new version of it.
  let correcting = $state<StoredObservation | null>(null);
  // Open histories by observation id. A late answer for a closed or changed entry is dropped.
  let histories = $state<Record<string, { status: number | null; versions: ObservationVersion[] }>>({});
  async function toggleHistory(o: StoredObservation) {
    if (histories[o.id]) {
      delete histories[o.id];
      return;
    }
    histories[o.id] = { status: null, versions: [] };
    const res = await loadHistory(patientId, o.id);
    if (!histories[o.id]) return;
    histories[o.id] = { status: res.status, versions: res.versions ?? [] };
  }
  let showTrend = $state(false);
  const trendList = $derived(showTrend ? trends(rows) : []);
  let erroring = $state<string | null>(null);
  let errorWhy = $state("");

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
    notices = res.notices;
    // A change makes an open history stale: close it rather than show an old list.
    histories = {};
  }
  $effect(() => {
    void patientId;
    void refresh;
    status = null;
    rows = [];
    notices = {};
    histories = {};
    measuredAt = nowInZone(new Date(), siteZone);
    void load();
  });

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(recordNow);
  }

  async function markError(o: StoredObservation) {
    await busy.run(async () => {
      message = null;
      if (!errorWhy.trim()) {
        message = STRINGS.vitalMarkErrorReasonNeeded;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "PUT",
        `/api/patients/${patientId}/observations/${o.id}`,
        { observation: enteredInErrorRecord(o), expected_version: o.version, reason: errorWhy.trim() },
      );
      const common = commonProblem(res.status);
      if (common) message = STRINGS[common];
      else if (res.status === 200) {
        message = STRINGS.vitalMarkedError;
        erroring = null;
        errorWhy = "";
        await load();
      } else if (res.status === 409) message = STRINGS.vitalConflict;
      else if (res.status === 422 && res.body?.errors) message = res.body.errors.join(" ");
      else message = STRINGS.error;
    });
  }

  function startCorrection(o: StoredObservation) {
    const v = VITALS.find((x) => x.loinc === o.code);
    if (!v) {
      message = STRINGS.vitalCannotCorrect;
      return;
    }
    correcting = o;
    message = null;
    vital = v.key;
    noValue = o.value === null || o.value === undefined;
    value = noValue ? "" : String(o.value);
    absentReason = o.absent_reason ?? "";
    low = o.range?.low != null ? String(o.range.low) : "";
    high = o.range?.high != null ? String(o.range.high) : "";
    measuredAt = nowInZone(new Date(o.effective_at), siteZone);
    why = "";
  }

  function cancelCorrection() {
    correcting = null;
    value = low = high = why = "";
    noValue = false;
    absentReason = "";
    measuredAt = nowInZone(new Date(), siteZone);
  }

  async function recordNow() {
    message = null;
    const built = formToRequest({ vital, value, noValue, absentReason, low, high, measuredAt, siteZone });
    if (!built.ok) {
      message = built.problem;
      return;
    }
    // The form has no critical range; a correction keeps the one the entry has rather than erasing it.
    if (correcting?.critical) built.record.critical = correcting.critical;
    if (correcting && !why.trim()) {
      message = STRINGS.vitalCorrectReasonNeeded;
      return;
    }
    const res = correcting
      ? await call<{ errors?: string[] }>(
          "PUT",
          `/api/patients/${patientId}/observations/${correcting.id}`,
          { observation: built.record, expected_version: correcting.version, reason: why.trim() },
        )
      : await call<{ errors?: string[] }>("POST", `/api/patients/${patientId}/observations`, {
          observation: built.record,
          reason: why.trim() || undefined,
        });
    const common = commonProblem(res.status);
    if (common) message = STRINGS[common];
    else if (res.status === 201 || (correcting && res.status === 200)) {
      message = correcting ? STRINGS.vitalCorrected : STRINGS.vitalsSaved;
      correcting = null;
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

{#snippet noticeList(o: StoredObservation)}
  {#each notices[o.id] ?? [] as n, i (i)}
    <p role="note">{noticeText(n)}</p>
  {/each}
{/snippet}

{#snippet historyList(o: StoredObservation)}
  {@const h = histories[o.id]}
  {#if h}
    {#if h.status === null}
      <p>{STRINGS.vitalHistoryLoading}</p>
    {:else if h.status === 403}
      <p>{STRINGS.vitalHistoryNotShown}</p>
    {:else if h.status !== 200}
      <p>{STRINGS.vitalHistoryCouldNot}</p>
    {:else}
      <ol aria-label={STRINGS.vitalHistory}>
        {#each h.versions as v (v.version)}
          <li>
            {STRINGS.vitalVersion.replace("{n}", String(v.version))}
            · {valueText(v)}
            · {assessmentText(v)}
            {#if rangeText(v.range)}· {STRINGS.vitalRange} {rangeText(v.range)}{/if}
            {#if v.status === "entered-in-error"}· {STRINGS.vitalHistoryInError}{/if}
            · {siteWhenText(v.recorded_at, siteZone)}
            · {STRINGS.vitalBy} <code>{v.author}</code>
            · {STRINGS.vitalHistoryReason} {v.reason || STRINGS.vitalHistoryNoReason}
          </li>
        {/each}
      </ol>
    {/if}
  {/if}
{/snippet}

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
            <button type="button" aria-expanded={!!histories[o.id]} onclick={() => toggleHistory(o)}>
              {histories[o.id] ? STRINGS.vitalHistoryHide : STRINGS.vitalHistory}
            </button>
            {@render noticeList(o)}
            {@render historyList(o)}
            {#if !correcting}
              <button type="button" onclick={() => startCorrection(o)}>{STRINGS.vitalCorrect}</button>
            {/if}
            {#if erroring === o.id}
              <label for="vs-err-{o.id}">{STRINGS.vitalMarkErrorWhy}</label>
              <input id="vs-err-{o.id}" bind:value={errorWhy} autocomplete="off" />
              <button type="button" disabled={busy.on} onclick={() => markError(o)}>
                {STRINGS.vitalMarkErrorConfirm}
              </button>
              <button type="button" onclick={() => ((erroring = null), (errorWhy = ""))}>
                {STRINGS.vitalMarkErrorCancel}
              </button>
            {:else}
              <button type="button" onclick={() => ((erroring = o.id), (errorWhy = ""))}>
                {STRINGS.vitalMarkError}
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if standing.length}
      <button type="button" aria-expanded={showTrend} onclick={() => (showTrend = !showTrend)}>
        {showTrend ? STRINGS.vitalTrendHide : STRINGS.vitalTrendShow}
      </button>
      {#if showTrend}
        <p>{STRINGS.vitalTrendNote}</p>
        {#each trendList as t (t.code)}
          <table>
            <caption>{t.name}</caption>
            <thead>
              <tr>
                <th scope="col">{STRINGS.vitalTrendTime}</th>
                <th scope="col">{STRINGS.vitalTrendValue}</th>
                <th scope="col">{STRINGS.vitalTrendWhere}</th>
              </tr>
            </thead>
            <tbody>
              {#each t.rows as o (o.id)}
                <tr>
                  <td><time datetime={o.effective_at}>{siteWhenText(o.effective_at, siteZone)}</time></td>
                  <td>{valueText(o)}</td>
                  <td>{assessmentText(o)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/each}
      {/if}
    {/if}
    {#if mistaken.length}
      <details>
        <summary>{STRINGS.vitalInError.replace("{n}", String(mistaken.length))}</summary>
        <ul>
          {#each mistaken as o (o.id)}
            <li>
              <s>{vitalName(o.code)} · {valueText(o)}</s>
              · <time datetime={o.effective_at}>{siteWhenText(o.effective_at, siteZone)}</time>
              <button type="button" aria-expanded={!!histories[o.id]} onclick={() => toggleHistory(o)}>
                {histories[o.id] ? STRINGS.vitalHistoryHide : STRINGS.vitalHistory}
              </button>
              {@render noticeList(o)}
              {@render historyList(o)}
            </li>
          {/each}
        </ul>
      </details>
    {/if}

    <h3>{correcting ? STRINGS.vitalCorrectTitle : STRINGS.vitalsRecordTitle}</h3>
    {#if correcting}
      <p>{STRINGS.vitalCorrectNote.replace("{n}", String(correcting.version))}</p>
    {/if}
    <form onsubmit={record}>
      <label for="vs-vital">{STRINGS.vitalLabel}</label>
      <select id="vs-vital" bind:value={vital} disabled={correcting !== null}>
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
      <button disabled={busy.on} type="submit">
        {correcting ? STRINGS.vitalCorrectSubmit : STRINGS.vitalSubmit}
      </button>
      {#if correcting}
        <button type="button" onclick={cancelCorrection}>{STRINGS.vitalMarkErrorCancel}</button>
      {/if}
    </form>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
