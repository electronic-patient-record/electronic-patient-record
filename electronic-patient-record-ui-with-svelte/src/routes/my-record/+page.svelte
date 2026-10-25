<script lang="ts">
  import { call, commonProblem } from "#lib/api.js";
  import { loadAllergies, type AllergyLoad } from "#lib/allergies.js";
  import PatientBanner from "#lib/PatientBanner.svelte";
  import { describe, whenText, type LogPage } from "#lib/access-log.js";
  import { patientToForm, type Stored } from "#lib/patient.js";
  import { STRINGS } from "#lib/strings.js";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE } from "#lib/locales.js";
  import { dataAge } from "#lib/data-age.js";

  type Entry = { relationship: "self" | "proxy"; patient: Stored; photo_version?: number | null };

  let records = $state<Entry[] | null>(null);
  let message = $state<string | null>(null);
  let loading = $state(true);
  let allergyById = $state<Record<string, AllergyLoad>>({});

  // EC-12: the page shows when the record was last read and says so when a later look failed. It
  // looks again once a minute while open, and keeps the records on screen if that fails.
  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let lastRead = $state<Date | null>(null);
  let lookFailed = $state(false);
  let now = $state(new Date());
  const age = $derived(dataAge(lastRead, now, lookFailed, locale));

  // This page is for the signed-in patient or proxy, so it always says so: the purpose of use is
  // `patient-access`, and the API fixes it for this endpoint too.
  async function load(refresh = false) {
    const res = await call<{ records: Entry[] }>(
      "GET",
      "/api/me/patients",
      undefined,
      "patient-access",
    );
    loading = false;
    if (refresh && res.status !== 200) {
      lookFailed = true;
      return;
    }
    const problem = commonProblem(res.status);
    if (problem) message = STRINGS[problem];
    else if (res.status === 200) {
      message = null;
      lastRead = new Date();
      lookFailed = false;
      records = res.body.records;
      // One allergy call per record, under the patient-access purpose.
      for (const r of res.body.records) {
        void loadAllergies(r.patient.id, "patient-access").then((l) => {
          allergyById[r.patient.id] = l;
        });
      }
    }
    else message = STRINGS.error;
  }

  $effect(() => {
    void load();
    const onVisible = () => {
      if (!document.hidden) void load(true);
    };
    document.addEventListener("visibilitychange", onVisible);
    const timer = setInterval(() => void load(true), 60_000);
    const tick = setInterval(() => (now = new Date()), 15_000);
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      clearInterval(timer);
      clearInterval(tick);
    };
  });

  // The access log is read on request, not on load: reading it is itself recorded.
  type Log = { entries: LogPage["entries"]; next: number | null; message: string | null };
  let logs = $state<Record<string, Log>>({});

  async function loadLog(id: string, before?: number) {
    const q = before === undefined ? "" : `?before=${before}`;
    const res = await call<LogPage>(
      "GET",
      `/api/me/patients/${id}/access-log${q}`,
      undefined,
      "patient-access",
    );
    const problem = commonProblem(res.status);
    const prior = before === undefined ? [] : (logs[id]?.entries ?? []);
    if (problem) logs[id] = { entries: prior, next: null, message: STRINGS[problem] };
    else if (res.status === 200)
      logs[id] = {
        entries: [...prior, ...res.body.entries],
        next: res.body.next,
        message: null,
      };
    else logs[id] = { entries: prior, next: null, message: STRINGS.error };
  }

  const shown = (v: string) => v || STRINGS.notAvailable;
  const fields = (s: Stored) => {
    const f = patientToForm(s.patient);
    return [
      [STRINGS.family, f.family],
      [STRINGS.given, f.given],
      [STRINGS.birthDate, f.birthDate],
      [STRINGS.sex, f.sex ? STRINGS.sexes[f.sex] : ""],
      [STRINGS.nhsNumber, f.nhsNumber],
      [STRINGS.postalCode, f.postalCode],
    ] as const;
  };
</script>

<svelte:head><title>{STRINGS.myRecordTitle}</title></svelte:head>

<h1>{STRINGS.myRecordTitle}</h1>
<p>{STRINGS.myRecordIntro}</p>

{#if loading}
  <p role="status" aria-busy="true">{STRINGS.loading}</p>
{/if}

{#if message}
  <p role="alert">{message}</p>
{/if}

{#if age}
  {#if age.stale}
    <p role="alert">{STRINGS.dataMayBeOld.replace("{time}", age.readAt)}</p>
  {:else}
    <p class="read-at">{STRINGS.dataReadAt.replace("{time}", age.readAt)}</p>
  {/if}
{/if}

{#if records && records.length === 0}
  <p role="status">{STRINGS.myRecordNone}</p>
{/if}

{#if records}
  {#each records as entry (entry.patient.id)}
    {@const log = logs[entry.patient.id]}
    <section aria-labelledby="rec-{entry.patient.id}">
      <h2 id="rec-{entry.patient.id}">
        {[entry.patient.patient.names[0]?.family, entry.patient.patient.names[0]?.given.join(" ")]
          .filter(Boolean)
          .join(", ")}
      </h2>
      <PatientBanner stored={{ ...entry.patient, photo_version: entry.photo_version }} purpose="patient-access" allergies={allergyById[entry.patient.id] ?? null} />
      {#if entry.relationship === "proxy"}<p>{STRINGS.myRecordProxy}</p>{/if}
      <dl>
        {#each fields(entry.patient) as [label, value] (label)}
          <dt>{label}</dt>
          <dd>{shown(value)}</dd>
        {/each}
      </dl>
      <p>{STRINGS.version}: {entry.patient.version}</p>
      <h3 id="log-{entry.patient.id}">{STRINGS.accessLogTitle}</h3>
      {#if !log}
        <button type="button" onclick={() => loadLog(entry.patient.id)}>
          {STRINGS.accessLogShow}
        </button>
      {:else}
        <div aria-live="polite">
          {#if log.message}<p role="alert">{log.message}</p>{/if}
          {#if log.entries.length === 0 && !log.message}
            <p>{STRINGS.accessLogNone}</p>
          {:else if log.entries.length > 0}
            <p>{STRINGS.accessLogNote}</p>
            <ul aria-labelledby="log-{entry.patient.id}">
              {#each log.entries as item (item.id)}
                <li><time datetime={item.at}>{whenText(item.at)}</time>: {describe(item)}</li>
              {/each}
            </ul>
          {/if}
          {#if log.next !== null}
            <button type="button" onclick={() => loadLog(entry.patient.id, log.next ?? undefined)}>
              {STRINGS.accessLogMore}
            </button>
          {/if}
        </div>
      {/if}
    </section>
  {/each}
{/if}
