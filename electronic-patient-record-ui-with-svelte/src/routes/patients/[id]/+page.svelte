<script lang="ts">
  import { page } from "$app/state";
  import { call, commonProblem, sendBytes } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import PatientBanner from "#lib/PatientBanner.svelte";
  import PatientFields from "#lib/PatientFields.svelte";
  import {
    emptyForm,
    formToPatient,
    patientToForm,
    type Stored,
  } from "#lib/patient.js";
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

  async function load() {
    loading = true;
    message = null;
    conflict = false;
    latest = null;
    const res = await call<Stored>("GET", `/api/patients/${id}`);
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
      form = patientToForm(res.body.patient);
      reason = "";
    } else {
      stored = null;
      message = STRINGS.error;
    }
  }

  $effect(() => {
    void id;
    void load();
  });

  // Someone else may edit while this page is open. The page looks again when the tab comes back
  // into view and once a minute while it is in view (each look is an audited read, so not more
  // often). Unsaved typing is never overwritten: if the form is dirty, the banner shows the newer
  // record and the page says so, and saving the old version would be refused anyway.
  let latest = $state<Stored | null>(null);
  const dirty = $derived(
    stored !== null &&
      (JSON.stringify(form) !== JSON.stringify(patientToForm(stored.patient)) ||
        reason.trim() !== ""),
  );

  async function look() {
    if (!stored || document.hidden || loading) return;
    const res = await call<Stored>("GET", `/api/patients/${stored.id}`);
    if (res.status !== 200 || !stored || res.body.version === stored.version) {
      return;
    }
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
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      clearInterval(timer);
    };
  });

  // The photo is its own record with its own version: uploading or removing it never touches the
  // identity fields being edited in the form.
  let photoFile = $state<File | null>(null);
  let photoReason = $state("");
  let photoMessage = $state<string | null>(null);
  const MAX_PHOTO = 512 * 1024;

  async function sendPhoto(withdraw: boolean) {
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
    const q = new URLSearchParams({
      expected_version: String(stored.photo_version ?? 0),
    });
    if (photoReason.trim()) q.set("reason", photoReason.trim());
    const res = await sendBytes<{ errors?: string[] }>(
      withdraw ? "POST" : "PUT",
      `/api/patients/${stored.id}/photo${withdraw ? "/withdraw" : ""}?${q}`,
      withdraw ? null : photoFile,
    );
    const common = commonProblem(res.status);
    if (common) photoMessage = STRINGS[common];
    else if (res.status === 200) {
      photoMessage = withdraw ? STRINGS.photoRemoved : STRINGS.photoSaved;
      photoFile = null;
      photoReason = "";
      // The photo version moved; read the record again, but keep what is being typed.
      const fresh = await call<Stored>("GET", `/api/patients/${stored.id}`);
      if (fresh.status === 200 && stored) {
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
    if (!stored) return;
    message = null;
    problems = [];
    saved = null;
    conflict = false;
    const res = await call<Stored & { errors?: string[] }>(
      "PUT",
      `/api/patients/${stored.id}`,
      {
        patient: formToPatient(form, stored.patient),
        expected_version: stored.version,
        reason: reason.trim() || undefined,
      },
    );
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
      problems = res.body.errors;
    } else {
      message = STRINGS.error;
    }
  }
</script>

<svelte:head><title>{STRINGS.patientTitle}</title></svelte:head>

<h1>{STRINGS.patientTitle}</h1>
<p><a href={localizePath("/patients", locale)}>{STRINGS.backToSearch}</a></p>

{#if loading}
  <p>{STRINGS.loading}</p>
{/if}

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if stored}
  <PatientBanner stored={latest ?? stored} />
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
    <button type="button" onclick={() => void sendPhoto(false)}>{STRINGS.photoUpload}</button>
    {#if stored.photo_version}
      <button type="button" onclick={() => void sendPhoto(true)}>{STRINGS.photoWithdraw}</button>
    {/if}
    {#if photoMessage}<p role="status">{photoMessage}</p>{/if}
  </section>
  <form onsubmit={save}>
    <PatientFields bind:form prefix="e" />
    <label for="e-reason">{STRINGS.reason}</label>
    <input id="e-reason" bind:value={reason} autocomplete="off" />
    <button type="submit">{STRINGS.save}</button>
  </form>
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
