<script lang="ts">
  // The banner every patient view carries (ID-7). It shows what is ON RECORD, never what is being
  // typed into a form, so a clinician always sees which person this is. Allergies are not held by
  // this system yet: the banner says so in words rather than showing nothing, because an empty
  // allergy line reads as "none".
  import {
    ageOn,
    NHS_NUMBER_SYSTEM,
    nhsDisplay,
    type Stored,
  } from "./patient.js";
  import { STRINGS } from "./strings.js";

  let { stored, today = new Date() }: { stored: Stored; today?: Date } =
    $props();

  const p = $derived(stored.patient);
  const name = $derived(
    [
      p.names[0]?.family?.toUpperCase(),
      (p.names[0]?.given ?? []).join(" "),
    ]
      .filter(Boolean)
      .join(", "),
  );
  const nhs = $derived(
    p.identifiers.find((i) => i.system === NHS_NUMBER_SYSTEM)?.value,
  );
  const other = $derived(
    p.identifiers.find((i) => i.system !== NHS_NUMBER_SYSTEM),
  );
  const age = $derived.by(() => {
    const a = ageOn(p.birth_date, today);
    if (!a) return STRINGS.bannerNoBirthDate;
    if (a.months !== undefined)
      return STRINGS.bannerAgeMonths.replace("{n}", String(a.months));
    return (a.approximate ? STRINGS.bannerAgeYearsApprox : STRINGS.bannerAgeYears).replace(
      "{n}",
      String(a.years),
    );
  });
</script>

<section class="banner" aria-label={STRINGS.bannerLabel}>
  <h2>{name || STRINGS.notAvailable}</h2>
  <p>
    {STRINGS.bannerBorn}: {p.birth_date ?? STRINGS.bannerNoBirthDate}
    · {STRINGS.bannerAge}: {age}
    {#if p.sex}· {STRINGS.sexes[p.sex]}{/if}
  </p>
  <p>{STRINGS.bannerNhs}: {nhs ? nhsDisplay(nhs) : STRINGS.bannerNone}</p>
  <p>
    {STRINGS.bannerOtherId}: {other ? `${other.system} ${other.value}` : STRINGS.bannerNone}
  </p>
  <p>{STRINGS.bannerPhoto}</p>
  <p class="allergies">
    {STRINGS.bannerAllergies}: {STRINGS.bannerAllergiesUnknown}
  </p>
</section>
