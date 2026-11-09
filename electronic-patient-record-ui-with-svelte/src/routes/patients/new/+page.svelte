<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import { newIdempotencyKey } from "#lib/idempotency.js";
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
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let form = $state(emptyForm());
  let reason = $state("");
  let overrideReason = $state("");
  let problems = $state<string[]>([]);
  let message = $state<string | null>(null);
  // One key for this registration: a double click or a retry stores one patient (OF-1).
  const idempotencyKey = newIdempotencyKey();
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
      message = TEXT.overrideNeeded;
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
    >(
      "POST",
      "/api/patients",
      {
        patient: formToPatient(override ? submitted : form),
        reason: reason.trim() || undefined,
        override_reason: override ? overrideReason.trim() : undefined,
      },
      undefined,
      { idempotencyKey },
    );
    const common = commonProblem(res.status);
    if (common) {
      message = TEXT[common];
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
      message = TEXT.error;
    }
  }

  const fieldName = (f: string) =>
    ({
      family: TEXT.family,
      given: TEXT.given,
      birthDate: TEXT.birthDate,
      sex: TEXT.sex,
      nhsNumber: TEXT.nhsNumber,
      postalCode: TEXT.postalCode,
    })[f] ?? f;
  const shown = (v: string) => v || TEXT.notAvailable;
</script>

<svelte:head><title>{TEXT.registerTitle}</title></svelte:head>

<h1>{TEXT.registerTitle}</h1>

<form
  onsubmit={(e) => {
    e.preventDefault();
    void register(false);
  }}
>
  <PatientFields bind:form prefix="n" />
  <label for="n-reason">{TEXT.reason}</label>
  <input id="n-reason" bind:value={reason} autocomplete="off" />
  <button type="submit" disabled={busy.on}>{TEXT.register}</button>
</form>

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if problems.length}
  <div role="alert">
    <h2>{TEXT.problems}</h2>
    <ul>
      {#each problems as p (p)}<li>{p}</li>{/each}
    </ul>
  </div>
{/if}

{#if candidates.length}
  <section aria-labelledby="dup-title" aria-live="polite">
    <h2 id="dup-title">{TEXT.duplicateTitle}</h2>
    <p>{TEXT.duplicateIntro}</p>
    {#if candidatesCut}<p>{TEXT.candidatesCut}</p>{/if}
    {#each candidates as c, i (isRestricted(c) ? `restricted-${i}` : c.existing.id)}
      {#if isRestricted(c)}
        <article class="candidate">
          <p>{TEXT.restrictedCandidate}</p>
        </article>
      {:else}
        <article class="candidate">
          <h3>
            {TEXT.decision}: {TEXT.decisions[c.decision]} ({TEXT.score}:
            {c.score.toFixed(2)})
          </h3>
          {#if c.deterministic}<p>{c.deterministic}</p>{/if}
          {#if c.conflicts.length}
            <p>{TEXT.conflicts}</p>
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
                <th>{TEXT.field}</th>
                <th>{TEXT.entered}</th>
                <th>{TEXT.existing}</th>
              </tr>
            </thead>
            <tbody>
              {#each comparisonRows(submitted, c.existing.patient) as row (row.field)}
                <tr>
                  <th scope="row">{fieldName(row.field)}</th>
                  <td>{shown(row.entered)}</td>
                  <td>
                    {shown(row.existing)}
                    {#if row.same}<span class="same">({TEXT.sameValue})</span
                      >{/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          <p>
            <a href={localizePath(`/patients/${c.existing.id}`, locale)}
              >{TEXT.openExisting}</a
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
      <label for="n-override">{TEXT.overrideReason}</label>
      <textarea id="n-override" rows="3" bind:value={overrideReason}></textarea>
      <button type="submit" disabled={busy.on}>{TEXT.registerAnyway}</button>
    </form>
  </section>
{/if}
