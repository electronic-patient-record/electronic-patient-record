<script lang="ts">
  import { call, commonProblem, fetchBlob } from "#lib/api.js";
  import { loadAllergies, type AllergyLoad } from "#lib/allergies.js";
  import PatientBanner from "#lib/PatientBanner.svelte";
  import {
    describe,
    LOG_PURPOSES,
    logCsvQuery,
    logQuery,
    readerZone,
    NO_FILTER,
    whenText,
    type LogFilter,
    type LogPage,
  } from "#lib/access-log.js";
  import { Busy } from "#lib/busy.svelte.js";
  import { logFormStrings } from "#lib/translations.js";
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
  // The filter the shown entries were read under, and the one being edited.
  let applied = $state<Record<string, LogFilter>>({});
  let editing = $state<Record<string, LogFilter>>({});
  const logBusy = new Busy();
  // The filter form's words, in the reader's locale (machine-written; English where missing).
  const F = $derived(logFormStrings(locale));
  // The patient's own days: the zone of the browser they are reading in.
  const zone = readerZone();

  async function loadLog(id: string, before?: number, filter: LogFilter = applied[id] ?? NO_FILTER) {
    await logBusy.run(() => loadLogNow(id, before, filter));
  }

  async function loadLogNow(id: string, before: number | undefined, filter: LogFilter) {
    editing[id] ??= { ...NO_FILTER };
    const q = logQuery(filter, before, zone);
    if (q === null) {
      logs[id] = { entries: logs[id]?.entries ?? [], next: logs[id]?.next ?? null, message: F.badDay };
      return;
    }
    const res = await call<LogPage>(
      "GET",
      `/api/me/patients/${id}/access-log${q}`,
      undefined,
      "patient-access",
    );
    if (res.status === 200) applied[id] = { ...filter };
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

  // The log as a file, under the filter that is showing (not one still being edited).
  async function downloadLog(id: string) {
    await logBusy.run(async () => {
      const q = logCsvQuery(applied[id] ?? NO_FILTER, zone);
      const res = q === null ? null : await fetchBlob(`/api/me/patients/${id}/access-log${q}`, "patient-access");
      if (!res || !res.body) {
        const problem = res ? commonProblem(res.status) : null;
        logs[id] = { entries: logs[id]?.entries ?? [], next: logs[id]?.next ?? null, message: problem ? STRINGS[problem] : STRINGS.accessLogDownloadFailed };
        return;
      }
      const url = URL.createObjectURL(res.body);
      const a = document.createElement("a");
      a.href = url;
      a.download = "access-log.csv";
      a.click();
      URL.revokeObjectURL(url);
    });
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
        <button type="button" disabled={logBusy.on} onclick={() => loadLog(entry.patient.id)}>
          {STRINGS.accessLogShow}
        </button>
      {:else}
        {@const f = editing[entry.patient.id] ?? { ...NO_FILTER }}
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void loadLog(entry.patient.id, undefined, editing[entry.patient.id] ?? NO_FILTER);
          }}
        >
          <fieldset>
            <legend>{F.filterTitle}</legend>
            <p>{F.zone.replace("{zone}", zone)}</p>
            <label for="lf-from-{entry.patient.id}">{F.from}</label>
            <input id="lf-from-{entry.patient.id}" type="date" bind:value={f.from} />
            <label for="lf-to-{entry.patient.id}">{F.to}</label>
            <input id="lf-to-{entry.patient.id}" type="date" bind:value={f.to} />
            <label for="lf-by-{entry.patient.id}">{F.by}</label>
            <select id="lf-by-{entry.patient.id}" bind:value={f.by}>
              <option value="">{F.byAny}</option>
              <option value="you">{F.byYou}</option>
              <option value="staff">{F.byStaff}</option>
            </select>
            <label for="lf-out-{entry.patient.id}">{F.outcome}</label>
            <select id="lf-out-{entry.patient.id}" bind:value={f.refused}>
              <option value="">{F.outcomeAny}</option>
              <option value="true">{F.refusedOnly}</option>
              <option value="false">{F.okOnly}</option>
            </select>
            <label for="lf-purpose-{entry.patient.id}">{F.purposeFilter}</label>
            <select id="lf-purpose-{entry.patient.id}" bind:value={f.purpose}>
              <option value="">{F.purposeAny}</option>
              {#each LOG_PURPOSES as code (code)}
                <option value={code}>{F.purposes[code]}</option>
              {/each}
            </select>
            <button type="submit" disabled={logBusy.on}>{F.apply}</button>
            <button
              type="button"
              disabled={logBusy.on}
              onclick={() => {
                editing[entry.patient.id] = { ...NO_FILTER };
                void loadLog(entry.patient.id, undefined, NO_FILTER);
              }}>{F.clear}</button
            >
            <button type="button" disabled={logBusy.on} onclick={() => void downloadLog(entry.patient.id)}>{F.download}</button>
          </fieldset>
        </form>
        <div aria-live="polite">
          {#if log.message}<p role="alert">{log.message}</p>{/if}
          {#if log.entries.length === 0 && !log.message}
            <p>{STRINGS.accessLogNone}</p>
          {:else if log.entries.length > 0}
            <p>{STRINGS.accessLogNote}</p>
            <ul aria-labelledby="log-{entry.patient.id}">
              {#each log.entries as item (item.id)}
                <li><time datetime={item.at}>{whenText(item.at, zone)}</time>: {describe(item)}</li>
              {/each}
            </ul>
          {/if}
          {#if log.next !== null}
            <button type="button" disabled={logBusy.on} onclick={() => loadLog(entry.patient.id, log.next ?? undefined)}>
              {STRINGS.accessLogMore}
            </button>
          {/if}
        </div>
      {/if}
    </section>
  {/each}
{/if}
