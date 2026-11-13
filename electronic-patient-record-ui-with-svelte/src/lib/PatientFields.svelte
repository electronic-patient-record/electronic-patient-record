<script lang="ts">
  import { TEXT } from "#lib/i18n.js";
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
  <label for="{prefix}-family">{TEXT.family}</label>
  <input id="{prefix}-family" bind:value={form.family} autocomplete="off" />
{/if}
{#if show("given")}
  <label for="{prefix}-given">{TEXT.given}</label>
  <input
    id="{prefix}-given"
    bind:value={form.given}
    autocomplete="off"
    aria-describedby="{prefix}-given-hint"
  />
  <small id="{prefix}-given-hint">{TEXT.givenHint}</small>
{/if}
{#if show("birthDate")}
  <label for="{prefix}-birth">{TEXT.birthDate}</label>
  <input
    id="{prefix}-birth"
    bind:value={form.birthDate}
    autocomplete="off"
    inputmode="numeric"
  />
{/if}
{#if show("sex")}
  <label for="{prefix}-sex">{TEXT.sex}</label>
  <select id="{prefix}-sex" bind:value={form.sex}>
    <option value="">{TEXT.sexUnset}</option>
    {#each SEXES as s (s)}<option value={s}>{TEXT.sexes[s]}</option>{/each}
  </select>
{/if}
{#if show("nationalNumber")}
  <label for="{prefix}-national-number">{TEXT.nationalNumber}</label>
  <input
    id="{prefix}-national-number"
    bind:value={form.nationalNumber}
    autocomplete="off"
    inputmode="numeric"
  />
{/if}
{#if show("postalCode")}
  <label for="{prefix}-postcode">{TEXT.postalCode}</label>
  <input
    id="{prefix}-postcode"
    bind:value={form.postalCode}
    autocomplete="off"
  />
{/if}
{#if show("temporaryIdentity")}
  <label for="{prefix}-temporary">
    <input
      id="{prefix}-temporary"
      type="checkbox"
      bind:checked={form.temporaryIdentity}
      aria-describedby="{prefix}-temporary-hint"
    />
    {TEXT.temporaryIdentity}
  </label>
  <small id="{prefix}-temporary-hint">{TEXT.temporaryIdentityHint}</small>
{/if}
