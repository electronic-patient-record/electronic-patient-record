<script lang="ts">
  import { untrack } from "svelte";
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { siteWhenText } from "#lib/access-log.js";
  import { allergyLine, loadAllergies, type AllergyLoad } from "#lib/allergies.js";
  import PatientBanner from "#lib/PatientBanner.svelte";
  import type { Stored } from "#lib/patient.js";
  import { latestReadings } from "#lib/summary.js";
  import { STRINGS } from "#lib/strings.js";
  import {
    assessmentText,
    loadObservations,
    noticeText,
    valueText,
    vitalName,
    type VitalsLoad,
  } from "#lib/vitals.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const id = $derived(page.params.id);

  let stored = $state<Stored | null>(null);
  let allergies = $state<AllergyLoad | null>(null);
  let vitals = $state<VitalsLoad | null>(null);
  let loading = $state(true);
  let message = $state<string | null>(null);

  // The page is reused when the address moves to another patient: every answer is checked against
  // the patient now on screen before it is used (CQ-6, hazard H-016).
  let ticket = 0;

  async function load() {
    const forId = id ?? "";
    const mine = ++ticket;
    stored = null;
    allergies = null;
    vitals = null;
    message = null;
    loading = true;
    const res = await call<Stored>("GET", `/api/patients/${forId}`);
    if (mine !== ticket) return;
    loading = false;
    const common = commonProblem(res.status);
    if (common) message = STRINGS[common];
    else if (res.status === 404) message = STRINGS.notFound;
    else if (res.status !== 200) message = STRINGS.error;
    else {
      stored = res.body;
      const [a, v] = await Promise.all([loadAllergies(forId), loadObservations(forId)]);
      if (mine !== ticket) return;
      allergies = a;
      vitals = v;
    }
  }

  $effect(() => {
    void id;
    untrack(() => void load());
  });

  const siteZone = $derived(stored?.site_time_zone ?? "UTC");
  const latest = $derived(vitals?.observations ? latestReadings(vitals.observations) : []);
</script>

<svelte:head><title>{STRINGS.summaryTitle}</title></svelte:head>

<h1>{STRINGS.summaryTitle}</h1>
<p><a href={localizePath(`/patients/${id}`, locale)}>{STRINGS.summaryBack}</a></p>

{#if loading}
  <p role="status" aria-busy="true">{STRINGS.loading}</p>
{/if}
{#if message}
  <p role="alert">{message}</p>
{/if}

{#if stored}
  <PatientBanner {stored} {allergies} />

  <section aria-labelledby="sum-allergies-h">
    <h2 id="sum-allergies-h">{STRINGS.summaryAllergies}</h2>
    {#if allergies === null}
      <p role="status">{STRINGS.loading}</p>
    {:else if allergies.status === 403}
      <p>{STRINGS.allergyNotShown}</p>
    {:else if allergies.status !== 200 || !allergies.data}
      <p role="alert">{STRINGS.allergyLoadFailed}</p>
    {:else}
      {@const data = allergies.data}
      {@const standing = data.allergies.filter((a) => a.allergy.status !== "entered-in-error")}
      {#if standing.length === 0}
        <p>{STRINGS.allergyNone}</p>
      {:else}
        <ul>
          {#each standing as a (a.id)}
            <li>
              {allergyLine(a)}
              {#each data.amendment_notices?.[a.id] ?? [] as n, i (i)}
                <p role="note">{noticeText(n)}</p>
              {/each}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>

  <section aria-labelledby="sum-vitals-h">
    <h2 id="sum-vitals-h">{STRINGS.summaryVitals}</h2>
    {#if vitals === null}
      <p role="status">{STRINGS.loading}</p>
    {:else if vitals.status === 403}
      <p>{STRINGS.summaryVitalsNotShown}</p>
    {:else if vitals.status !== 200 || !vitals.observations}
      <p role="alert">{STRINGS.summaryVitalsFailed}</p>
    {:else if latest.length === 0}
      <p>{STRINGS.summaryVitalsNone}</p>
    {:else}
      <p>{STRINGS.summaryVitalsNote}</p>
      <ul>
        {#each latest as o (o.id)}
          <li>
            {vitalName(o.code)} · {valueText(o)} · {assessmentText(o)} ·
            <time datetime={o.effective_at}>{siteWhenText(o.effective_at, siteZone)}</time>
            {#each vitals.notices[o.id] ?? [] as n, i (i)}
              <p role="note">{noticeText(n)}</p>
            {/each}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section aria-labelledby="sum-todo-h">
    <h2 id="sum-todo-h">{STRINGS.summaryNotBuilt}</h2>
    <p>{STRINGS.summaryNotBuiltNote}</p>
    <ul>
      {#each Object.values(STRINGS.summaryNotBuiltItems) as name (name)}
        <li>{name}</li>
      {/each}
    </ul>
  </section>
{/if}
