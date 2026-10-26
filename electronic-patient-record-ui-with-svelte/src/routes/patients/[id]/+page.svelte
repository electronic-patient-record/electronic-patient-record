<script lang="ts">
  import { untrack } from "svelte";
  import { page } from "$app/state";
  import { call, commonProblem, sendBytes } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import AllergySection from "#lib/AllergySection.svelte";
  import BreakGlassSection from "#lib/BreakGlassSection.svelte";
  import { whenText } from "#lib/access-log.js";
  import { loadAllergies, type AllergyLoad } from "#lib/allergies.js";
  import { patientProblemList } from "#lib/conflicts.js";
  import MergeSection from "#lib/MergeSection.svelte";
  import PatientBanner from "#lib/PatientBanner.svelte";
  import PatientFields from "#lib/PatientFields.svelte";
  import {
    emptyForm,
    formToPatient,
    patientToForm,
    type Stored,
  } from "#lib/patient.js";
  import { dataAge } from "#lib/data-age.js";
  import { STRINGS } from "#lib/strings.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const id = $derived(page.params.id);

  let stored = $state<Stored | null>(null);
  let form = $state(emptyForm());
  let reason = $state("");
  let loading = $state(true);
  let message = $state<string | null>(null);
  let problems = $state<string[]>([]);
  let conflict = $state(false);
  let saved = $state<string | null>(null);

  // One allergy call per look, shared by the banner and the allergy section: each is an audited
  // read, and two views of the same answer must not disagree.
  let allergies = $state<AllergyLoad | null>(null);
  // When emergency access opened on this page ends, kept after the form has gone.
  let glassUntil = $state<string | null>(null);

  // Wrong-patient guard (CQ-6, hazard H-016): the page is reused when the address moves to another
  // patient, so every answer is checked against the patient now on screen before it is used, and the
  // previous patient's state is cleared at once rather than left showing until the next answer.
  let loadTicket = 0;
  let allergyTicket = 0;
  const current = (forId: string | undefined) => forId === id;

  async function reloadAllergies(background = false) {
    const forId = id;
    const ticket = ++allergyTicket;
    const res = await loadAllergies(forId ?? "", undefined, { background });
    if (current(forId) && ticket === allergyTicket) allergies = res;
  }

  async function load() {
    const forId = id;
    const ticket = ++loadTicket;
    if (stored?.id !== forId) {
      stored = null;
      allergies = null;
      form = emptyForm();
      reason = "";
      glassUntil = null;
      saved = null;
      problems = [];
      photoFile = null;
      photoReason = "";
      photoMessage = null;
    }
    loading = true;
    message = null;
    conflict = false;
    latest = null;
    lastRead = null;
    lookFailed = false;
    const res = await call<Stored>("GET", `/api/patients/${forId}`);
    if (!current(forId) || ticket !== loadTicket) return;
    loading = false;
    const common = commonProblem(res.status);
    if (common) {
      stored = null;
      message = STRINGS[common];
    } else if (res.status === 404) {
      stored = null;
      message = STRINGS.notFound;
    } else if (res.status === 200) {
      stored = res.body;
      lastRead = new Date();
      form = patientToForm(res.body.patient);
      reason = "";
      allergies = null;
      void reloadAllergies();
    } else {
      stored = null;
      message = STRINGS.error;
    }
  }

  // Load when, and only when, the patient changes: `load` reads page state, which must not become
  // a dependency of this effect (each answer would start another load).
  $effect(() => {
    void id;
    untrack(() => void load());
  });

  // Someone else may edit while this page is open. The page looks again when the tab comes back
  // into view and once a minute while it is in view (each look is an audited read, so not more
  // often). Unsaved typing is never overwritten: if the form is dirty, the banner shows the newer
  // record and the page says so, and saving the old version would be refused anyway.
  let latest = $state<Stored | null>(null);
  // When the record on screen was last read, and whether the latest background look failed (EC-12).
  let lastRead = $state<Date | null>(null);
  let lookFailed = $state(false);
  let now = $state(new Date());
  const age = $derived(dataAge(lastRead, now, lookFailed, locale));
  const dirty = $derived(
    stored !== null &&
      (JSON.stringify(form) !== JSON.stringify(patientToForm(stored.patient)) ||
        reason.trim() !== ""),
  );

  async function look() {
    if (!stored || document.hidden || loading) return;
    const forId = stored.id;
    // Allergies can change without the identity record changing, so they are looked at too. Both
    // are background looks: nobody asked for them, so they do not keep the session alive (CQ-8).
    void reloadAllergies(true);
    const res = await call<Stored>("GET", `/api/patients/${forId}`, undefined, undefined, {
      background: true,
    });
    if (!stored || stored.id !== forId || !current(forId)) return;
    if (res.status !== 200) {
      lookFailed = true;
      return;
    }
    lastRead = new Date();
    lookFailed = false;
    if (res.body.version === stored.version) return;
    if (dirty) {
      latest = res.body;
    } else {
      stored = res.body;
      form = patientToForm(res.body.patient);
      latest = null;
    }
  }

  $effect(() => {
    const onVisible = () => {
      if (!document.hidden) void look();
    };
    document.addEventListener("visibilitychange", onVisible);
    const timer = setInterval(() => void look(), 60_000);
    const tick = setInterval(() => (now = new Date()), 15_000);
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      clearInterval(timer);
      clearInterval(tick);
    };
  });

  // The photo is its own record with its own version: uploading or removing it never touches the
  // identity fields being edited in the form.
  let photoFile = $state<File | null>(null);
  let photoReason = $state("");
  let photoMessage = $state<string | null>(null);
  const MAX_PHOTO = 512 * 1024;
  // One write at a time: a second click while the first is in flight is ignored, not sent.
  let saving = $state(false);
  let photoBusy = $state(false);

  async function sendPhoto(withdraw: boolean) {
    if (!stored || photoBusy) return;
    photoBusy = true;
    try {
      await sendPhotoNow(withdraw);
    } finally {
      photoBusy = false;
    }
  }

  async function sendPhotoNow(withdraw: boolean) {
    if (!stored) return;
    photoMessage = null;
    if (!withdraw) {
      if (!photoFile) {
        photoMessage = STRINGS.photoPick;
        return;
      }
      if (photoFile.size > MAX_PHOTO) {
        photoMessage = STRINGS.photoTooBig;
        return;
      }
      if (!["image/png", "image/jpeg"].includes(photoFile.type)) {
        photoMessage = STRINGS.photoWrongType;
        return;
      }
    }
    const forId = stored.id;
    const q = new URLSearchParams({
      expected_version: String(stored.photo_version ?? 0),
    });
    if (photoReason.trim()) q.set("reason", photoReason.trim());
    const res = await sendBytes<{ errors?: string[] }>(
      withdraw ? "POST" : "PUT",
      `/api/patients/${forId}/photo${withdraw ? "/withdraw" : ""}?${q}`,
      withdraw ? null : photoFile,
    );
    if (stored?.id !== forId) return;
    const common = commonProblem(res.status);
    if (common) photoMessage = STRINGS[common];
    else if (res.status === 200) {
      photoMessage = withdraw ? STRINGS.photoRemoved : STRINGS.photoSaved;
      photoFile = null;
      photoReason = "";
      // The photo version moved; read the record again, but keep what is being typed.
      const fresh = await call<Stored>("GET", `/api/patients/${forId}`);
      if (fresh.status === 200 && stored?.id === forId) {
        stored = { ...stored, photo_version: fresh.body.photo_version };
        if (latest) latest = { ...latest, photo_version: fresh.body.photo_version };
      }
    } else if (res.status === 409) conflict = true;
    else if (res.status === 422 && res.body?.errors)
      photoMessage = res.body.errors.join(" ");
    else photoMessage = STRINGS.error;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!stored || saving) return;
    saving = true;
    try {
      await saveNow(stored.id, stored.version, stored.patient);
    } finally {
      saving = false;
    }
  }

  async function saveNow(forId: string, version: number, base: Stored["patient"]) {
    message = null;
    problems = [];
    saved = null;
    conflict = false;
    const res = await call<Stored & { errors?: string[]; error_codes?: unknown }>(
      "PUT",
      `/api/patients/${forId}`,
      {
        patient: formToPatient(form, base),
        expected_version: version,
        reason: reason.trim() || undefined,
      },
    );
    if (stored?.id !== forId) return;
    const common = commonProblem(res.status);
    if (common) {
      message = STRINGS[common];
    } else if (res.status === 200) {
      stored = res.body;
      form = patientToForm(res.body.patient);
      reason = "";
      saved = STRINGS.saved.replace("{v}", String(res.body.version));
    } else if (res.status === 409) {
      conflict = true;
    } else if (res.status === 422 && res.body?.errors) {
      problems = patientProblemList(res.body.errors, res.body.error_codes, locale);
    } else {
      message = STRINGS.error;
    }
  }
</script>

<svelte:head><title>{STRINGS.patientTitle}</title></svelte:head>

<h1>{STRINGS.patientTitle}</h1>
<p><a href={localizePath("/patients", locale)}>{STRINGS.backToSearch}</a></p>
<p>
  <a href={localizePath(`/patients/${id}/access`, locale)}>{STRINGS.accessManage}</a>
</p>

{#if loading}
  <p role="status" aria-busy="true">{STRINGS.loading}</p>
{/if}

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if stored}
  <PatientBanner stored={latest ?? stored} {allergies} />
  {#if age}
    {#if age.stale}
      <p role="alert">{STRINGS.dataMayBeOld.replace("{time}", age.readAt)}</p>
    {:else}
      <p class="read-at">{STRINGS.dataReadAt.replace("{time}", age.readAt)}</p>
    {/if}
  {/if}
  {#if stored.replaced_by}
    <div role="alert">
      <p>{STRINGS.mergedNotice}</p>
      <p>
        <a href={localizePath(`/patients/${stored.replaced_by}`, locale)}>
          {STRINGS.mergedOpen}
        </a>
      </p>
    </div>
  {/if}
  {#if latest}
    <div role="alert">
      <p>{STRINGS.changedElsewhere.replace("{v}", String(latest.version))}</p>
      <button type="button" onclick={() => void load()}>
        {STRINGS.reloadDiscard}
      </button>
    </div>
  {/if}
  <p>
    {STRINGS.version}: {stored.version} · {STRINGS.recordedBy}: {stored.author}
  </p>
  <!-- A record merged into another is read-only: its changes go to the surviving record. -->
  {#if !stored.replaced_by}
  {#if allergies?.status === 403 && allergies.offer}
    <BreakGlassSection
      patientId={stored.id}
      onopened={async (until) => {
        glassUntil = until;
        await reloadAllergies();
      }}
    />
  {/if}
  {#if glassUntil}
    <p role="status">
      {STRINGS.glassOpened.replace("{until}", whenText(glassUntil))}
    </p>
  {/if}
  {#if allergies?.status === 200 && allergies.data}
    <AllergySection
      patientId={stored.id}
      data={allergies.data}
      onchange={() => reloadAllergies()}
    />
  {/if}
  {#if !stored.replaced_by}
    <MergeSection
      patientId={stored.id}
      replaces={stored.replaces ?? []}
      onchange={load}
    />
  {/if}
  <section aria-labelledby="photo-h">
    <h2 id="photo-h">{STRINGS.photoTitle}</h2>
    <label for="photo-file">{STRINGS.photoFile}</label>
    <input
      id="photo-file"
      type="file"
      accept="image/png,image/jpeg"
      onchange={(e) => (photoFile = e.currentTarget.files?.[0] ?? null)}
    />
    <label for="photo-why">{STRINGS.photoWhy}</label>
    <input id="photo-why" bind:value={photoReason} autocomplete="off" />
    <button type="button" disabled={photoBusy} onclick={() => void sendPhoto(false)}>{STRINGS.photoUpload}</button>
    {#if stored.photo_version}
      <button type="button" disabled={photoBusy} onclick={() => void sendPhoto(true)}>{STRINGS.photoWithdraw}</button>
    {/if}
    {#if photoMessage}<p role="status">{photoMessage}</p>{/if}
  </section>
  <form onsubmit={save}>
    <PatientFields bind:form prefix="e" />
    <label for="e-reason">{STRINGS.reason}</label>
    <input id="e-reason" bind:value={reason} autocomplete="off" />
    <button type="submit" disabled={saving}>{STRINGS.save}</button>
  </form>
  {/if}
{/if}

{#if saved}
  <p role="status">{saved}</p>
{/if}

{#if conflict}
  <div role="alert">
    <p>{STRINGS.conflictChanged}</p>
    <button type="button" onclick={() => void load()}>{STRINGS.reload}</button>
  </div>
{/if}

{#if problems.length}
  <div role="alert">
    <h2>{STRINGS.problems}</h2>
    <ul>
      {#each problems as p (p)}<li>{p}</li>{/each}
    </ul>
  </div>
{/if}
