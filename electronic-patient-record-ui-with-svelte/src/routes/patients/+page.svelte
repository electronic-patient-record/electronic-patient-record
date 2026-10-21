<script lang="ts">
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
  import { STRINGS } from "#lib/strings.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let form = $state(emptyForm());
  let found = $state<Stored[] | null>(null);
  let limit = $state(50);
  let message = $state<string | null>(null);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    message = null;
    const query = searchQuery(form);
    if (!query) {
      found = null;
      message = STRINGS.searchNothing;
      return;
    }
    const res = await call<{ results: Stored[]; limit: number }>(
      "GET",
      `/api/patients?${query}`,
    );
    const problem = commonProblem(res.status);
    if (problem) {
      found = null;
      message = STRINGS[problem];
    } else if (res.status === 200) {
      found = res.body.results;
      limit = res.body.limit;
    } else {
      found = null;
      message = STRINGS.error;
    }
  }

  const label = (s: Stored) => {
    const f = patientToForm(s.patient);
    return [f.family, f.given].filter(Boolean).join(", ") || STRINGS.notAvailable;
  };
</script>

<svelte:head><title>{STRINGS.searchTitle}</title></svelte:head>

<h1>{STRINGS.searchTitle}</h1>
<p>{STRINGS.searchHint}</p>

<form onsubmit={submit}>
  <PatientFields bind:form prefix="s" only={["family", "birthDate", "nhsNumber"]} />
  <button type="submit">{STRINGS.search}</button>
</form>
<p>
  <a href={localizePath("/patients/new", locale)}>{STRINGS.registerLink}</a> ·
  <a href={localizePath("/patients/compare", locale)}>{STRINGS.compareTitle}</a>
</p>

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if found}
  <section aria-live="polite">
    <h2>{STRINGS.results}</h2>
    {#if found.length === 0}
      <p>{STRINGS.noResults}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>{STRINGS.family} / {STRINGS.given}</th>
            <th>{STRINGS.birthDate}</th>
            <th>{STRINGS.nhsNumber}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each found as s (s.id)}
            {@const f = patientToForm(s.patient)}
            <tr>
              <td>{label(s)}</td>
              <td>{f.birthDate || STRINGS.notAvailable}</td>
              <td>{f.nhsNumber || STRINGS.notAvailable}</td>
              <td>
                <a href={localizePath(`/patients/${s.id}`, locale)}
                  >{STRINGS.open}<span class="sr-only"> {label(s)}</span></a
                >
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if found.length >= limit}
        <p>{STRINGS.resultsCapped.replace("{n}", String(limit))}</p>
      {/if}
    {/if}
  </section>
{/if}
