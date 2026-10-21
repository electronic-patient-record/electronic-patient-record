<script lang="ts">
  import { call, commonProblem } from "#lib/api.js";
  import { patientToForm, type Stored } from "#lib/patient.js";
  import { STRINGS } from "#lib/strings.js";

  type Entry = { relationship: "self" | "proxy"; patient: Stored };

  let records = $state<Entry[] | null>(null);
  let message = $state<string | null>(null);
  let loading = $state(true);

  // This page is for the signed-in patient or proxy, so it always says so: the purpose of use is
  // `patient-access`, and the API fixes it for this endpoint too.
  async function load() {
    const res = await call<{ records: Entry[] }>(
      "GET",
      "/api/me/patients",
      undefined,
      "patient-access",
    );
    loading = false;
    const problem = commonProblem(res.status);
    if (problem) message = STRINGS[problem];
    else if (res.status === 200) records = res.body.records;
    else message = STRINGS.error;
  }

  $effect(() => {
    void load();
  });

  const shown = (v: string) => v || STRINGS.notAvailable;
  const fields = (s: Stored) => {
    const f = patientToForm(s.patient);
    return [
      [STRINGS.family, f.family],
      [STRINGS.given, f.given],
      [STRINGS.birthDate, f.birthDate],
      [STRINGS.sex, f.sex ? STRINGS.sexes[f.sex] : ""],
      [STRINGS.nhsNumber, f.nhsNumber],
      [STRINGS.postalCode, f.postalCode],
    ] as const;
  };
</script>

<svelte:head><title>{STRINGS.myRecordTitle}</title></svelte:head>

<h1>{STRINGS.myRecordTitle}</h1>
<p>{STRINGS.myRecordIntro}</p>

{#if loading}
  <p>{STRINGS.loading}</p>
{/if}

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if records && records.length === 0}
  <p role="status">{STRINGS.myRecordNone}</p>
{/if}

{#if records}
  {#each records as entry (entry.patient.id)}
    <section aria-labelledby="rec-{entry.patient.id}">
      <h2 id="rec-{entry.patient.id}">
        {[entry.patient.patient.names[0]?.family, entry.patient.patient.names[0]?.given.join(" ")]
          .filter(Boolean)
          .join(", ")}
      </h2>
      {#if entry.relationship === "proxy"}<p>{STRINGS.myRecordProxy}</p>{/if}
      <dl>
        {#each fields(entry.patient) as [label, value] (label)}
          <dt>{label}</dt>
          <dd>{shown(value)}</dd>
        {/each}
      </dl>
      <p>{STRINGS.version}: {entry.patient.version}</p>
    </section>
  {/each}
{/if}
