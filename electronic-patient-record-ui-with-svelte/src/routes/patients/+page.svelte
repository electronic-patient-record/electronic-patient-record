<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import PatientFields from "#lib/PatientFields.svelte";
  import {
    emptyForm,
    patientToForm,
    searchQuery,
    type Stored,
  } from "#lib/patient.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let form = $state(emptyForm());
  let found = $state<Stored[] | null>(null);
  // The next page, when the API says more may follow (`Link: rel="next"`). The API fills each page past patients whose
  // refusal of disclosure hides them, so a short page says nothing about who is hidden.
  let next = $state<string | null>(null);
  let message = $state<string | null>(null);

  const busy = new Busy();

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(searchNow);
  }

  async function searchNow() {
    message = null;
    next = null;
    const query = searchQuery(form);
    if (!query) {
      found = null;
      message = TEXT.searchNothing;
      return;
    }
    const res = await call<{ results: Stored[]; limit: number }>(
      "GET",
      `/api/patients?${query}`,
    );
    const problem = commonProblem(res.status);
    if (problem) {
      found = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      found = res.body.results;
      next = res.next ?? null;
    } else {
      found = null;
      message = TEXT.error;
    }
  }

  async function more() {
    await busy.run(moreNow);
  }

  // The next page is added below the ones shown. A failure leaves them, and says so.
  async function moreNow() {
    if (!next || !found) return;
    const res = await call<{ results: Stored[]; limit: number }>("GET", next);
    const problem = commonProblem(res.status);
    if (problem) message = TEXT[problem];
    else if (res.status === 200) {
      const shown = new Set(found.map((s) => s.id));
      found = [...found, ...res.body.results.filter((s) => !shown.has(s.id))];
      next = res.next ?? null;
    } else message = TEXT.error;
  }

  const label = (s: Stored) => {
    const f = patientToForm(s.patient);
    return [f.family, f.given].filter(Boolean).join(", ") || TEXT.notAvailable;
  };
</script>

<svelte:head><title>{TEXT.searchTitle}</title></svelte:head>

<h1>{TEXT.searchTitle}</h1>
<p>{TEXT.searchHint}</p>

<form onsubmit={submit}>
  <PatientFields
    bind:form
    prefix="s"
    only={["family", "birthDate", "nationalNumber"]}
  />
  <button type="submit" disabled={busy.on}>{TEXT.search}</button>
</form>
<p>
  <a href={localizePath("/patients/new", locale)}>{TEXT.registerLink}</a> ·
  <a href={localizePath("/patients/compare", locale)}>{TEXT.compareTitle}</a>
</p>

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if found}
  <section aria-live="polite">
    <h2>{TEXT.results}</h2>
    {#if found.length === 0}
      <p>{TEXT.noResults}</p>
    {:else}
      <!-- The roles are kept when the layout is not a table (XP-2, `app.css`). -->
      <!-- svelte-ignore a11y_no_redundant_roles -->
      <table class="stack" role="table">
        <thead role="rowgroup">
          <tr role="row">
            <th scope="col" role="columnheader">
              {TEXT.family} / {TEXT.given}
            </th>
            <th scope="col" role="columnheader">{TEXT.birthDate}</th>
            <th scope="col" role="columnheader">{TEXT.nationalNumber}</th>
            <th scope="col" role="columnheader"></th>
          </tr>
        </thead>
        <tbody role="rowgroup">
          {#each found as s (s.id)}
            {@const f = patientToForm(s.patient)}
            <tr role="row">
              <td role="cell" data-label={`${TEXT.family} / ${TEXT.given}`}>
                {label(s)}
                {#if f.temporaryIdentity}
                  <strong>({TEXT.temporaryIdentity})</strong>
                {/if}
              </td>
              <td role="cell" data-label={TEXT.birthDate}>
                {f.birthDate || TEXT.notAvailable}
              </td>
              <td role="cell" data-label={TEXT.nationalNumber}>
                {f.nationalNumber || TEXT.notAvailable}
              </td>
              <td role="cell">
                <a href={localizePath(`/patients/${s.id}`, locale)}>
                  {TEXT.open}<span class="sr-only"> {label(s)}</span>
                </a>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if next}
        <p>{TEXT.resultsCapped}</p>
        <button type="button" onclick={more} disabled={busy.on}>
          {TEXT.searchMore}
        </button>
      {/if}
    {/if}
  </section>
{/if}
