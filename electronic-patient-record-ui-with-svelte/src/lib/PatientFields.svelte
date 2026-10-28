<script lang="ts">
  import { STRINGS } from "#lib/strings.js";
  import { SEXES, type PatientForm } from "#lib/patient.js";

  // The identity fields shared by search, register and edit. `prefix` keeps the element ids
  // unique when a page shows two of these.
  let {
    form = $bindable(),
    prefix,
    only = undefined,
  }: {
    form: PatientForm;
    prefix: string;
    only?: (keyof PatientForm)[];
  } = $props();
  const show = (f: keyof PatientForm) => !only || only.includes(f);
</script>

{#if show("family")}
  <label for="{prefix}-family">{STRINGS.family}</label>
  <input id="{prefix}-family" bind:value={form.family} autocomplete="off" />
{/if}
{#if show("given")}
  <label for="{prefix}-given">{STRINGS.given}</label>
  <input
    id="{prefix}-given"
    bind:value={form.given}
    autocomplete="off"
    aria-describedby="{prefix}-given-hint"
  />
  <small id="{prefix}-given-hint">{STRINGS.givenHint}</small>
{/if}
{#if show("birthDate")}
  <label for="{prefix}-birth">{STRINGS.birthDate}</label>
  <input
    id="{prefix}-birth"
    bind:value={form.birthDate}
    autocomplete="off"
    inputmode="numeric"
  />
{/if}
{#if show("sex")}
  <label for="{prefix}-sex">{STRINGS.sex}</label>
  <select id="{prefix}-sex" bind:value={form.sex}>
    <option value="">{STRINGS.sexUnset}</option>
    {#each SEXES as s (s)}<option value={s}>{STRINGS.sexes[s]}</option>{/each}
  </select>
{/if}
{#if show("nhsNumber")}
  <label for="{prefix}-nhs">{STRINGS.nhsNumber}</label>
  <input
    id="{prefix}-nhs"
    bind:value={form.nhsNumber}
    autocomplete="off"
    inputmode="numeric"
  />
{/if}
{#if show("postalCode")}
  <label for="{prefix}-postcode">{STRINGS.postalCode}</label>
  <input
    id="{prefix}-postcode"
    bind:value={form.postalCode}
    autocomplete="off"
  />
{/if}
