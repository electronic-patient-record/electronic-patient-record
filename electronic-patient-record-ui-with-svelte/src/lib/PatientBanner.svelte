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
  import { bannerAllergies, type AllergyLoad } from "./allergies.js";
  import { fetchBlob } from "./api.js";
  import { STRINGS } from "./strings.js";

  // `purpose` is the purpose of use the photo is fetched under: "my record" passes
  // `patient-access`; staff pages leave it to the picker.
  let {
    stored,
    today = new Date(),
    purpose,
    allergies = null,
  }: {
    stored: Stored;
    today?: Date;
    purpose?: string;
    // What the allergy call returned, or null while it is still loading. The page makes the call
    // (once) and hands it to the banner and the allergy section.
    allergies?: AllergyLoad | null;
  } = $props();
  const allergyLine = $derived(bannerAllergies(allergies));

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
  {#if stored.drill}
    <p class="drill" role="alert">{STRINGS.bannerDrill}</p>
  {/if}
  <h2>{name || STRINGS.notAvailable}</h2>
  {#if stored.replaced_by}
    <p class="merged" role="alert">{STRINGS.bannerMerged}</p>
  {/if}
  <p>
    {STRINGS.bannerBorn}: {p.birth_date ?? STRINGS.bannerNoBirthDate}
    · {STRINGS.bannerAge}: {age}
    {#if p.sex}· {STRINGS.sexes[p.sex]}{/if}
  </p>
  <p>{STRINGS.bannerNhs}: {nhs ? nhsDisplay(nhs) : STRINGS.bannerNone}</p>
  <p>
    {STRINGS.bannerOtherId}: {other ? `${other.system} ${other.value}` : STRINGS.bannerNone}
  </p>
  {#if photoUrl}
    <img class="photo" src={photoUrl} alt={STRINGS.bannerPhotoAlt.replace("{name}", name)} />
  {:else if photoFailed}
    <p role="status">{STRINGS.bannerPhotoFailed}</p>
  {:else}
    <p>{STRINGS.bannerPhoto}</p>
  {/if}
  <p class="allergies" class:high={allergyLine.high}>
    {STRINGS.bannerAllergies}: {allergyLine.text}
  </p>
</section>
