<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import PatientFields from "#lib/PatientFields.svelte";
  import {
    comparisonRows,
    emptyForm,
    formToPatient,
    isRestricted,
    type Candidate,
    type PatientForm,
    type Stored,
  } from "#lib/patient.js";
  import { conflictList, patientProblemList } from "#lib/conflicts.js";
  import { STRINGS } from "#lib/strings.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let form = $state(emptyForm());
  let reason = $state("");
  let overrideReason = $state("");
  let problems = $state<string[]>([]);
  let message = $state<string | null>(null);
  let candidates = $state<Candidate[]>([]);
  // More records matched than the server compares; only the closest were (CQ-36).
  let candidatesCut = $state(false);
  // What was submitted, so the side-by-side view does not change while the form is edited.
  let submitted = $state<PatientForm>(emptyForm());
  const busy = new Busy();

  async function register(override: boolean) {
    await busy.run(() => registerNow(override));
  }

  async function registerNow(override: boolean) {
    message = null;
    problems = [];
    if (override && !overrideReason.trim()) {
      message = STRINGS.overrideNeeded;
      return;
    }
    if (!override) {
      candidates = [];
      candidatesCut = false;
      submitted = { ...form };
    }
    const res = await call<
      Stored & {
        errors?: string[];
        error_codes?: unknown;
        candidates?: Candidate[];
        candidates_cut?: boolean;
      }
    >("POST", "/api/patients", {
      patient: formToPatient(override ? submitted : form),
      reason: reason.trim() || undefined,
      override_reason: override ? overrideReason.trim() : undefined,
    });
    const common = commonProblem(res.status);
    if (common) {
      message = STRINGS[common];
    } else if (res.status === 201) {
      await goto(localizePath(`/patients/${res.body.id}`, locale));
    } else if (res.status === 409 && res.body.candidates) {
      candidates = res.body.candidates;
      candidatesCut = res.body.candidates_cut === true;
    } else if (res.status === 422 && res.body.errors) {
      problems = patientProblemList(
        res.body.errors,
        res.body.error_codes,
        locale,
      );
    } else {
      message = STRINGS.error;
    }
  }

  const fieldName = (f: string) =>
    ({
      family: STRINGS.family,
      given: STRINGS.given,
      birthDate: STRINGS.birthDate,
      sex: STRINGS.sex,
      nhsNumber: STRINGS.nhsNumber,
      postalCode: STRINGS.postalCode,
    })[f] ?? f;
  const shown = (v: string) => v || STRINGS.notAvailable;
</script>

<svelte:head><title>{STRINGS.registerTitle}</title></svelte:head>

<h1>{STRINGS.registerTitle}</h1>

<form
  onsubmit={(e) => {
    e.preventDefault();
    void register(false);
  }}
>
  <PatientFields bind:form prefix="n" />
  <label for="n-reason">{STRINGS.reason}</label>
  <input id="n-reason" bind:value={reason} autocomplete="off" />
  <button type="submit" disabled={busy.on}>{STRINGS.register}</button>
</form>

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if problems.length}
  <div role="alert">
    <h2>{STRINGS.problems}</h2>
    <ul>
      {#each problems as p (p)}<li>{p}</li>{/each}
    </ul>
  </div>
{/if}

{#if candidates.length}
  <section aria-labelledby="dup-title" aria-live="polite">
    <h2 id="dup-title">{STRINGS.duplicateTitle}</h2>
    <p>{STRINGS.duplicateIntro}</p>
    {#if candidatesCut}<p>{STRINGS.candidatesCut}</p>{/if}
    {#each candidates as c, i (isRestricted(c) ? `restricted-${i}` : c.existing.id)}
      {#if isRestricted(c)}
        <article class="candidate">
          <p>{STRINGS.restrictedCandidate}</p>
        </article>
      {:else}
        <article class="candidate">
          <h3>
            {STRINGS.decision}: {STRINGS.decisions[c.decision]} ({STRINGS.score}:
            {c.score.toFixed(2)})
          </h3>
          {#if c.deterministic}<p>{c.deterministic}</p>{/if}
          {#if c.conflicts.length}
            <p>{STRINGS.conflicts}</p>
            <ul>
              {#each conflictList(c.conflicts, c.conflict_codes, locale) as why, i (i)}<li
                >
                  {why}
                </li>{/each}
            </ul>
          {/if}
          <table>
            <thead>
              <tr>
                <th>{STRINGS.field}</th>
                <th>{STRINGS.entered}</th>
                <th>{STRINGS.existing}</th>
              </tr>
            </thead>
            <tbody>
              {#each comparisonRows(submitted, c.existing.patient) as row (row.field)}
                <tr>
                  <th scope="row">{fieldName(row.field)}</th>
                  <td>{shown(row.entered)}</td>
                  <td>
                    {shown(row.existing)}
                    {#if row.same}<span class="same">({STRINGS.sameValue})</span
                      >{/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          <p>
            <a href={localizePath(`/patients/${c.existing.id}`, locale)}
              >{STRINGS.openExisting}</a
            >
          </p>
        </article>
      {/if}
    {/each}

    <form
      onsubmit={(e) => {
        e.preventDefault();
        void register(true);
      }}
    >
      <label for="n-override">{STRINGS.overrideReason}</label>
      <textarea id="n-override" rows="3" bind:value={overrideReason}></textarea>
      <button type="submit" disabled={busy.on}>{STRINGS.registerAnyway}</button>
    </form>
  </section>
{/if}
