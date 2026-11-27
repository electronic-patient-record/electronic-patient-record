<script lang="ts">
  // The banner every patient view carries (ID-7). It shows what is ON RECORD, never what is being
  // typed into a form, so a clinician always sees which person this is. Allergies are not held by
  // this system yet: the banner says so in words rather than showing nothing, because an empty
  // allergy line reads as "none".
  import {
    ageOn,
    NATIONAL_NUMBER_SYSTEM,
    nationalNumberDisplay,
    type Stored,
  } from "./patient.js";
  import {
    bannerAllergies,
    stripAllergies,
    type AllergyLoad,
  } from "./allergies.js";
  import { fetchBlob } from "./api.js";
  import { bannerDeath, type DeathLoad } from "./death.js";
  import { bannerAdjustments, type AdjustmentLoad } from "./adjustments.js";
  import { bannerRefusals, type RefusalLoad } from "./treatment-refusals.js";
  import { TEXT } from "./i18n.js";

  // `purpose` is the purpose of use the photo is fetched under: "my record" passes
  // `patient-access`; staff pages leave it to the picker.
  let {
    stored,
    today = new Date(),
    purpose,
    allergies = null,
    death = null,
    adjustments = null,
    refusals = null,
  }: {
    stored: Stored;
    today?: Date;
    purpose?: string;
    // What the allergy call returned, or null while it is still loading. The page makes the call
    // (once) and hands it to the banner and the allergy section.
    allergies?: AllergyLoad | null;
    // What the death call returned. Staff pages pass it; a patient's own view does not ask.
    death?: DeathLoad | null;
    // What the adjustment call returned. Staff pages pass it; a patient's own view does not ask.
    adjustments?: AdjustmentLoad | null;
    // What the refusal-of-treatment call returned (RF-7). Staff pages pass it; a patient's own view does not ask.
    refusals?: RefusalLoad | null;
  } = $props();
  const refusalLine = $derived(bannerRefusals(refusals));
  const deathLine = $derived(bannerDeath(death));
  const needsLine = $derived(bannerAdjustments(adjustments));
  const allergyLine = $derived(
    bannerAllergies(allergies, stored.site_time_zone),
  );

  // The photo is fetched with the page's own call (so it carries the purpose of use) and shown from
  // an object URL that is revoked when the photo changes or the banner goes. The server serves it
  // `no-store` and `nosniff`, and only PNG or JPEG bytes are ever stored.
  let photoUrl = $state<string | null>(null);
  let photoFailed = $state(false);
  $effect(() => {
    const id = stored.id;
    const version = stored.photo_version;
    photoUrl = null;
    photoFailed = false;
    if (!version) return;
    let url: string | null = null;
    let stale = false;
    void fetchBlob(`/api/patients/${id}/photo`, purpose).then((res) => {
      if (stale) return;
      if (res.status === 200 && res.body) {
        url = URL.createObjectURL(res.body);
        photoUrl = url;
      } else {
        photoFailed = true;
      }
    });
    return () => {
      stale = true;
      if (url) URL.revokeObjectURL(url);
    };
  });

  // A small or zoomed screen has no room for the whole banner to stay (H-070): the banner scrolls with the page and
  // this strip stays: whose record it is, what it must not be taken for, and allergies. Hidden from a screen reader,
  // which has the banner itself; and not shown at all on a large screen, where the banner stays.
  const firstSentence = (text: string) => text.split(/[.。।]\s/)[0] ?? text;
  const strip = $derived({
    allergies: stripAllergies(allergies, stored.site_time_zone),
    // A refusal stays in sight, as allergies do; so does a list that could not be read.
    refusals: refusalLine.show && refusalLine.loud ? refusalLine : null,
    flags: [
      ...(stored.drill ? [firstSentence(TEXT.bannerDrill)] : []),
      ...(deathLine.kind === "asserted" ? [deathLine.head] : []),
      ...(deathLine.kind === "unknown" ? [deathLine.text] : []),
      ...(stored.patient.temporary_identity
        ? [firstSentence(TEXT.bannerTemporaryIdentity)]
        : []),
      ...(stored.replaced_by ? [firstSentence(TEXT.bannerMerged)] : []),
    ],
  });

  // How tall the strip is, for what is jumped to or gets the focus to stop below it (`app.css`). Through the style
  // object and not a `style` attribute, which the CSP forbids.
  let stripHeight = $state(0);
  $effect(() => {
    const root = document.documentElement.style;
    root.setProperty("--strip-height", `${stripHeight}px`);
    return () => root.removeProperty("--strip-height");
  });

  const p = $derived(stored.patient);
  const name = $derived(
    [p.names[0]?.family?.toUpperCase(), (p.names[0]?.given ?? []).join(" ")]
      .filter(Boolean)
      .join(", "),
  );
  const national = $derived(
    p.identifiers.find((i) => i.system === NATIONAL_NUMBER_SYSTEM)?.value,
  );
  const other = $derived(
    p.identifiers.find((i) => i.system !== NATIONAL_NUMBER_SYSTEM),
  );
  const age = $derived.by(() => {
    const a = ageOn(p.birth_date, today, stored.site_time_zone);
    if (!a) return TEXT.bannerNoBirthDate;
    if (a.months !== undefined)
      return TEXT.bannerAgeMonths.replace("{n}", String(a.months));
    return (
      a.approximate ? TEXT.bannerAgeYearsApprox : TEXT.bannerAgeYears
    ).replace("{n}", String(a.years));
  });
</script>

<section class="banner" aria-label={TEXT.bannerLabel}>
  {#if stored.drill}
    <p class="drill" role="alert">{TEXT.bannerDrill}</p>
  {/if}
  {#if deathLine.kind === "asserted"}
    <p class="drill" role="alert">{deathLine.text}</p>
  {:else if deathLine.kind === "unknown"}
    <p role="status">{deathLine.text}</p>
  {/if}
  {#if p.temporary_identity}
    <p class="drill" role="alert">{TEXT.bannerTemporaryIdentity}</p>
  {/if}
  <h2>{name || TEXT.notAvailable}</h2>
  {#if stored.replaced_by}
    <p class="merged" role="alert">{TEXT.bannerMerged}</p>
  {/if}
  <p>
    {TEXT.bannerBorn}: {p.birth_date ?? TEXT.bannerNoBirthDate}
    · {TEXT.bannerAge}: {age}
    {#if p.sex}· {TEXT.sexes[p.sex]}{/if}
  </p>
  {#if needsLine.kind === "needs"}
    <p><strong>{needsLine.text}</strong></p>
    {#if needsLine.overdue}<p>{needsLine.overdue}</p>{/if}
  {:else if needsLine.kind === "status"}
    <p>{needsLine.text}</p>
  {/if}
  <p>
    {TEXT.bannerNationalNumber}: {national
      ? nationalNumberDisplay(national)
      : TEXT.bannerNone}
  </p>
  <p>
    {TEXT.bannerOtherId}: {other
      ? `${other.system} ${other.value}`
      : TEXT.bannerNone}
  </p>
  {#if photoUrl}
    <img
      class="photo"
      src={photoUrl}
      alt={TEXT.bannerPhotoAlt.replace("{name}", name)}
    />
  {:else if photoFailed}
    <p role="status">{TEXT.bannerPhotoFailed}</p>
  {:else}
    <p>{TEXT.bannerPhoto}</p>
  {/if}
  <p class="allergies" class:high={allergyLine.high}>
    {TEXT.bannerAllergies}: {allergyLine.text}
  </p>
  {#if refusalLine.show}
    <p class="refusals" class:high={refusalLine.loud}>
      {TEXT.bannerRefusals}: {refusalLine.text}
    </p>
  {/if}
</section>

<div class="banner-strip" aria-hidden="true" bind:offsetHeight={stripHeight}>
  {#if strip.flags.length > 0}
    <p class="flag">{strip.flags.join(" · ")}</p>
  {/if}
  <p>
    <strong>{name || TEXT.notAvailable}</strong>
    · {p.birth_date ?? TEXT.bannerNoBirthDate}
    · {national
      ? nationalNumberDisplay(national)
      : other
        ? `${other.system} ${other.value}`
        : TEXT.bannerNone}
  </p>
  <p class="allergies" class:high={strip.allergies.high}>
    {TEXT.bannerAllergies}: {strip.allergies.text}
  </p>
  {#if strip.refusals}
    <p class="refusals high">
      {TEXT.bannerRefusals}: {strip.refusals.text}
    </p>
  {/if}
</div>
