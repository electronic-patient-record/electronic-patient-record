<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { page } from "$app/state";
  import { conflictList, type ConflictCode } from "#lib/conflicts.js";
  import { DEFAULT_LOCALE } from "#lib/locales.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);

  type Field = {
    field: string;
    similarity: number | null;
    weight: number;
    note: string;
  };
  type Result = {
    score: number;
    decision: keyof typeof TEXT.decisions;
    fields: Field[];
    deterministic: string | null;
    conflicts: string[];
    conflict_codes?: ConflictCode[];
  };

  let a = $state({ family: "", given: "", birth: "" });
  let b = $state({ family: "", given: "", birth: "" });
  let result = $state<Result | null>(null);
  let failed = $state(false);
  let signInRequired = $state(false);
  let forbidden = $state(false);

  const patient = (p: { family: string; given: string; birth: string }) => ({
    names: [{ family: p.family || null, given: p.given ? [p.given] : [] }],
    birth_date: p.birth || null,
  });

  const busy = new Busy();

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(compareNow);
  }

  async function compareNow() {
    failed = false;
    signInRequired = false;
    forbidden = false;
    try {
      const res = await fetch("/api/proxy/api/patients/compare", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ a: patient(a), b: patient(b) }),
      });
      if (res.status === 401) {
        signInRequired = true;
        result = null;
        return;
      }
      if (res.status === 403) {
        forbidden = true;
        result = null;
        return;
      }
      if (!res.ok) throw new Error(String(res.status));
      result = (await res.json()) as Result;
    } catch {
      result = null;
      failed = true;
    }
  }
</script>

<svelte:head><title>{TEXT.compareTitle}</title></svelte:head>

<h1>{TEXT.compareTitle}</h1>

<form onsubmit={submit}>
  <div class="pair">
    {#each [{ id: "a", label: TEXT.patientA, model: a }, { id: "b", label: TEXT.patientB, model: b }] as p (p.id)}
      <fieldset>
        <legend>{p.label}</legend>
        <label for="{p.id}-family">{TEXT.family}</label>
        <input
          id="{p.id}-family"
          bind:value={p.model.family}
          autocomplete="off"
        />
        <label for="{p.id}-given">{TEXT.given}</label>
        <input
          id="{p.id}-given"
          bind:value={p.model.given}
          autocomplete="off"
        />
        <label for="{p.id}-birth">{TEXT.birthDate}</label>
        <input
          id="{p.id}-birth"
          bind:value={p.model.birth}
          autocomplete="off"
          inputmode="numeric"
        />
      </fieldset>
    {/each}
  </div>
  <button type="submit" disabled={busy.on}>{TEXT.compare}</button>
</form>

{#if forbidden}
  <p role="alert">{TEXT.forbidden}</p>
{/if}

{#if signInRequired}
  <p role="alert">{TEXT.signInRequired}</p>
{/if}

{#if failed}
  <p role="alert">{TEXT.error}</p>
{/if}

{#if result}
  <section aria-live="polite">
    <h2>{TEXT.decision}: {TEXT.decisions[result.decision]}</h2>
    <p>{TEXT.score}: {result.score.toFixed(2)}</p>
    {#if result.deterministic}<p>{result.deterministic}</p>{/if}
    {#if result.conflicts.length}
      <h3>{TEXT.conflicts}</h3>
      <ul>
        {#each conflictList(result.conflicts, result.conflict_codes, locale) as c, i (i)}<li
          >
            {c}
          </li>{/each}
      </ul>
    {/if}
    <table>
      <thead>
        <tr><th>{TEXT.field}</th><th>{TEXT.similarity}</th><th></th></tr>
      </thead>
      <tbody>
        {#each result.fields as f (f.field)}
          <tr>
            <td>{f.field}</td>
            <td
              >{f.similarity === null
                ? TEXT.notAvailable
                : f.similarity.toFixed(2)}</td
            >
            <td>{f.note}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
{/if}
