<script lang="ts">
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import PatientBanner from "#lib/PatientBanner.svelte";
  import PatientFields from "#lib/PatientFields.svelte";
  import {
    emptyForm,
    formToPatient,
    patientToForm,
    type Stored,
  } from "#lib/patient.js";
  import { STRINGS } from "#lib/strings.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const id = $derived(page.params.id);

  let stored = $state<Stored | null>(null);
  let form = $state(emptyForm());
  let reason = $state("");
  let loading = $state(true);
  let message = $state<string | null>(null);
  let problems = $state<string[]>([]);
  let conflict = $state(false);
  let saved = $state<string | null>(null);

  async function load() {
    loading = true;
    message = null;
    conflict = false;
    const res = await call<Stored>("GET", `/api/patients/${id}`);
    loading = false;
    const common = commonProblem(res.status);
    if (common) {
      stored = null;
      message = STRINGS[common];
    } else if (res.status === 404) {
      stored = null;
      message = STRINGS.notFound;
    } else if (res.status === 200) {
      stored = res.body;
      form = patientToForm(res.body.patient);
      reason = "";
    } else {
      stored = null;
      message = STRINGS.error;
    }
  }

  $effect(() => {
    void id;
    void load();
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!stored) return;
    message = null;
    problems = [];
    saved = null;
    conflict = false;
    const res = await call<Stored & { errors?: string[] }>(
      "PUT",
      `/api/patients/${stored.id}`,
      {
        patient: formToPatient(form, stored.patient),
        expected_version: stored.version,
        reason: reason.trim() || undefined,
      },
    );
    const common = commonProblem(res.status);
    if (common) {
      message = STRINGS[common];
    } else if (res.status === 200) {
      stored = res.body;
      form = patientToForm(res.body.patient);
      reason = "";
      saved = STRINGS.saved.replace("{v}", String(res.body.version));
    } else if (res.status === 409) {
      conflict = true;
    } else if (res.status === 422 && res.body?.errors) {
      problems = res.body.errors;
    } else {
      message = STRINGS.error;
    }
  }
</script>

<svelte:head><title>{STRINGS.patientTitle}</title></svelte:head>

<h1>{STRINGS.patientTitle}</h1>
<p><a href={localizePath("/patients", locale)}>{STRINGS.backToSearch}</a></p>

{#if loading}
  <p>{STRINGS.loading}</p>
{/if}

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if stored}
  <PatientBanner {stored} />
  <p>
    {STRINGS.version}: {stored.version} · {STRINGS.recordedBy}: {stored.author}
  </p>
  <form onsubmit={save}>
    <PatientFields bind:form prefix="e" />
    <label for="e-reason">{STRINGS.reason}</label>
    <input id="e-reason" bind:value={reason} autocomplete="off" />
    <button type="submit">{STRINGS.save}</button>
  </form>
{/if}

{#if saved}
  <p role="status">{saved}</p>
{/if}

{#if conflict}
  <div role="alert">
    <p>{STRINGS.conflictChanged}</p>
    <button type="button" onclick={() => void load()}>{STRINGS.reload}</button>
  </div>
{/if}

{#if problems.length}
  <div role="alert">
    <h2>{STRINGS.problems}</h2>
    <ul>
      {#each problems as p (p)}<li>{p}</li>{/each}
    </ul>
  </div>
{/if}
