<script lang="ts">
  import { STRINGS } from "#lib/strings.js";

  type Field = { field: string; similarity: number | null; weight: number; note: string };
  type Result = {
    score: number;
    decision: keyof typeof STRINGS.decisions;
    fields: Field[];
    deterministic: string | null;
    conflicts: string[];
  };

  let a = $state({ family: "", given: "", birth: "" });
  let b = $state({ family: "", given: "", birth: "" });
  let result = $state<Result | null>(null);
  let failed = $state(false);

  const patient = (p: { family: string; given: string; birth: string }) => ({
    names: [{ family: p.family || null, given: p.given ? [p.given] : [] }],
    birth_date: p.birth || null,
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    failed = false;
    try {
      const res = await fetch("/api/proxy/api/patients/compare", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ a: patient(a), b: patient(b) }),
      });
      if (!res.ok) throw new Error(String(res.status));
      result = (await res.json()) as Result;
    } catch {
      result = null;
      failed = true;
    }
  }
</script>

<svelte:head><title>{STRINGS.compareTitle}</title></svelte:head>

<h1>{STRINGS.compareTitle}</h1>

<form onsubmit={submit}>
  <div class="pair">
    {#each [{ id: "a", label: STRINGS.patientA, model: a }, { id: "b", label: STRINGS.patientB, model: b }] as p (p.id)}
      <fieldset>
        <legend>{p.label}</legend>
        <label for="{p.id}-family">{STRINGS.family}</label>
        <input id="{p.id}-family" bind:value={p.model.family} autocomplete="off" />
        <label for="{p.id}-given">{STRINGS.given}</label>
        <input id="{p.id}-given" bind:value={p.model.given} autocomplete="off" />
        <label for="{p.id}-birth">{STRINGS.birthDate}</label>
        <input id="{p.id}-birth" bind:value={p.model.birth} autocomplete="off" inputmode="numeric" />
      </fieldset>
    {/each}
  </div>
  <button type="submit">{STRINGS.compare}</button>
</form>

{#if failed}
  <p role="alert">{STRINGS.error}</p>
{/if}

{#if result}
  <section aria-live="polite">
    <h2>{STRINGS.decision}: {STRINGS.decisions[result.decision]}</h2>
    <p>{STRINGS.score}: {result.score.toFixed(2)}</p>
    {#if result.deterministic}<p>{result.deterministic}</p>{/if}
    {#if result.conflicts.length}
      <h3>{STRINGS.conflicts}</h3>
      <ul>
        {#each result.conflicts as c (c)}<li>{c}</li>{/each}
      </ul>
    {/if}
    <table>
      <thead>
        <tr><th>{STRINGS.field}</th><th>{STRINGS.similarity}</th><th></th></tr>
      </thead>
      <tbody>
        {#each result.fields as f (f.field)}
          <tr>
            <td>{f.field}</td>
            <td>{f.similarity === null ? STRINGS.notAvailable : f.similarity.toFixed(2)}</td>
            <td>{f.note}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
{/if}
