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
  let limit = $state(50);
  let message = $state<string | null>(null);

  const busy = new Busy();

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(searchNow);
  }

  async function searchNow() {
    message = null;
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
      limit = res.body.limit;
    } else {
      found = null;
      message = TEXT.error;
    }
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
    only={["family", "birthDate", "nhsNumber"]}
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
      <table>
        <thead>
          <tr>
            <th>{TEXT.family} / {TEXT.given}</th>
            <th>{TEXT.birthDate}</th>
            <th>{TEXT.nhsNumber}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each found as s (s.id)}
            {@const f = patientToForm(s.patient)}
            <tr>
              <td>
                {label(s)}
                {#if f.temporaryIdentity}
                  <strong>({TEXT.temporaryIdentity})</strong>
                {/if}
              </td>
              <td>{f.birthDate || TEXT.notAvailable}</td>
              <td>{f.nhsNumber || TEXT.notAvailable}</td>
              <td>
                <a href={localizePath(`/patients/${s.id}`, locale)}
                  >{TEXT.open}<span class="sr-only"> {label(s)}</span></a
                >
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if found.length >= limit}
        <p>{TEXT.resultsCapped.replace("{n}", String(limit))}</p>
      {/if}
    {/if}
  </section>
{/if}
