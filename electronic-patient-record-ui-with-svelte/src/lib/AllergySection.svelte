<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // The allergy list and the forms to record, end or retract one, and to state "no known
  // allergies" (RC-5, in part). Shown only to a role that may read allergies. Nothing is deleted:
  // ending an allergy or marking it entered in error is a new version, and entered in error is final.
  import { call, commonProblem } from "./api.js";
  import type {
    AllergyData,
    AllergyRecord,
    Category,
    Criticality,
    StoredAllergy,
  } from "./allergies.js";
  import { STRINGS } from "./strings.js";
  import { noticeText } from "./vitals.js";

  const busy = new Busy();
  let {
    patientId,
    data,
    onchange,
  }: {
    patientId: string;
    data: AllergyData;
    // Called after any change, so the page reloads the list and the banner.
    onchange: () => void | Promise<void>;
  } = $props();

  let substance = $state("");
  let category = $state<Category | "">("");
  let criticality = $state<Criticality | "">("");
  let reaction = $state("");
  let onset = $state("");
  let why = $state("");
  let message = $state<string | null>(null);

  const label = (a: StoredAllergy) =>
    [
      a.allergy.substance,
      a.allergy.criticality
        ? `(${STRINGS.allergyCriticalities[a.allergy.criticality]})`
        : "",
    ]
      .filter(Boolean)
      .join(" ");

  async function done(res: { status: number; body: unknown }) {
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = STRINGS[common];
    else if (res.status === 200 || res.status === 201) {
      message = STRINGS.allergySaved;
      await onchange();
    } else if (res.status === 409) {
      message = STRINGS.allergyChanged;
      await onchange();
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = STRINGS.error;
  }

  async function add(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => addNow());
  }

  async function addNow() {
    message = null;
    const allergy: AllergyRecord = {
      substance: substance.trim(),
      status: "active",
      ...(category ? { category } : {}),
      ...(criticality ? { criticality } : {}),
      ...(reaction.trim() ? { reaction: reaction.trim() } : {}),
      ...(onset.trim() ? { onset: onset.trim() } : {}),
    };
    const res = await call("POST", `/api/patients/${patientId}/allergies`, {
      allergy,
      reason: why.trim() || undefined,
    });
    if (res.status === 201) {
      substance = category = criticality = reaction = onset = why = "";
    }
    await done(res);
  }

  async function change(
    a: StoredAllergy,
    status: "inactive" | "entered-in-error",
  ) {
    await busy.run(() => changeNow(a, status));
  }

  async function changeNow(
    a: StoredAllergy,
    status: "inactive" | "entered-in-error",
  ) {
    message = null;
    await done(
      await call("PUT", `/api/patients/${patientId}/allergies/${a.id}`, {
        allergy: { ...a.allergy, status },
        expected_version: a.version,
        reason: why.trim() || undefined,
      }),
    );
  }

  async function statement(noneKnown: boolean) {
    await busy.run(() => statementNow(noneKnown));
  }

  async function statementNow(noneKnown: boolean) {
    message = null;
    await done(
      await call("PUT", `/api/patients/${patientId}/allergy-statement`, {
        none_known: noneKnown,
        expected_version: data.statement?.version ?? 0,
        reason: why.trim() || undefined,
      }),
    );
  }
</script>

<section aria-labelledby="allergies-h">
  <h2 id="allergies-h">{STRINGS.allergiesTitle}</h2>
  {#if data.allergies.length === 0}
    <p>{STRINGS.allergyNone}</p>
  {:else}
    <ul>
      {#each data.allergies as a (a.id)}
        <li>
          <strong>{label(a)}</strong>
          · {STRINGS.allergyStatusLabels[a.allergy.status]}
          {#if a.allergy.reaction}· {a.allergy.reaction}{/if}
          {#if a.allergy.onset}· {a.allergy.onset}{/if}
          {#each data.amendment_notices?.[a.id] ?? [] as n, i (i)}
            <p role="note">{noticeText(n)}</p>
          {/each}
          {#if a.allergy.status === "active"}
            <button
              disabled={busy.on}
              type="button"
              onclick={() => void change(a, "inactive")}
            >
              {STRINGS.allergyEnd}
            </button>
          {/if}
          {#if a.allergy.status !== "entered-in-error"}
            <button
              disabled={busy.on}
              type="button"
              onclick={() => void change(a, "entered-in-error")}
            >
              {STRINGS.allergyMistake}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <form onsubmit={add}>
    <label for="al-substance">{STRINGS.allergySubstance}</label>
    <input
      id="al-substance"
      bind:value={substance}
      required
      maxlength="200"
      autocomplete="off"
    />
    <label for="al-category">{STRINGS.allergyCategory}</label>
    <select id="al-category" bind:value={category}>
      <option value=""></option>
      {#each Object.entries(STRINGS.allergyCategories) as [k, v] (k)}
        <option value={k}>{v}</option>
      {/each}
    </select>
    <label for="al-crit">{STRINGS.allergyCriticality}</label>
    <select id="al-crit" bind:value={criticality}>
      <option value=""></option>
      {#each Object.entries(STRINGS.allergyCriticalities) as [k, v] (k)}
        <option value={k}>{v}</option>
      {/each}
    </select>
    <label for="al-reaction">{STRINGS.allergyReaction}</label>
    <input
      id="al-reaction"
      bind:value={reaction}
      maxlength="500"
      autocomplete="off"
    />
    <label for="al-onset">{STRINGS.allergyOnset}</label>
    <input id="al-onset" bind:value={onset} autocomplete="off" />
    <label for="al-why">{STRINGS.allergyWhy}</label>
    <input id="al-why" bind:value={why} autocomplete="off" />
    <button disabled={busy.on} type="submit">{STRINGS.allergyAdd}</button>
  </form>

  <p>
    {#if data.statement?.no_known_allergies}
      <button
        disabled={busy.on}
        type="button"
        onclick={() => void statement(false)}
      >
        {STRINGS.allergyWithdrawNone}
      </button>
    {:else}
      <button
        disabled={busy.on}
        type="button"
        onclick={() => void statement(true)}
      >
        {STRINGS.allergyNoneKnown}
      </button>
    {/if}
  </p>
  {#if message}<p role="status">{message}</p>{/if}
</section>
