<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // A patient asks for a vital sign in their own record to be corrected, sees the state of each
  // request, withdraws one before it is decided, and states that they disagree with a refusal (RC-4,
  // H-035). The purpose is `patient-access`. Who is asking is the token. A statement is kept as
  // written and shown as text.
  import { call, commonProblem } from "./api.js";
  import { siteWhenText } from "./access-log.js";
  import {
    aboutText,
    askBody,
    disagreeBody,
    dueText,
    loadAmendments,
    ownStepsFor,
    stateText,
    targetLines,
    type AmendmentView,
    type TargetKind,
  } from "./amendments.js";
  import { allergyLine, loadAllergies, type StoredAllergy } from "./allergies.js";
  import { STRINGS } from "./strings.js";
  import { loadObservations, vitalLine, type StoredObservation } from "./vitals.js";

  const PURPOSE = "patient-access";
  const busy = new Busy();
  let { patientId, siteZone = "UTC" }: { patientId: string; siteZone?: string } = $props();

  let status = $state<number | null>(null);
  let rows = $state<AmendmentView[]>([]);
  let readings = $state<StoredObservation[] | null>(null);
  let allergies = $state<StoredAllergy[] | null>(null);
  let message = $state<string | null>(null);

  let reading = $state("");
  let text = $state("");
  let stating = $state<string | null>(null);
  let statement = $state("");

  const standing = $derived((readings ?? []).filter((o) => o.status !== "entered-in-error"));
  const standingAllergies = $derived(
    (allergies ?? []).filter((a) => a.allergy.status !== "entered-in-error"),
  );
  const lines = $derived(targetLines(readings ?? [], allergies ?? [], siteZone));

  async function load() {
    const [a, o, g] = await Promise.all([
      loadAmendments(patientId, { purpose: PURPOSE }),
      loadObservations(patientId, { purpose: PURPOSE }),
      loadAllergies(patientId, PURPOSE),
    ]);
    status = a.status;
    rows = a.amendments ?? [];
    readings = o.observations;
    allergies = g.data?.allergies ?? null;
  }
  $effect(() => {
    void patientId;
    void load();
  });

  function report(res: { status: number; body: { errors?: string[] } | null }, ok: number, done: string) {
    const common = commonProblem(res.status);
    if (common) message = STRINGS[common];
    else if (res.status === ok) message = done;
    else if (res.status === 409) message = STRINGS.amendConflict;
    else if (res.status === 422 && res.body?.errors) message = res.body.errors.join(" ");
    else message = STRINGS.error;
    return res.status === ok;
  }

  async function ask(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(async () => {
      message = null;
      // The choice is `kind:id`.
      const [kind, id] = reading.split(":") as [TargetKind, string];
      const found =
        kind === "Observation"
          ? standing.find((x) => x.id === id)
          : standingAllergies.find((x) => x.id === id);
      const built = askBody(found ? { kind, id: found.id, version: found.version } : null, text);
      if (!built.ok) {
        message = built.problem;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "POST",
        `/api/patients/${patientId}/amendments`,
        built.body,
        PURPOSE,
      );
      if (report(res, 201, STRINGS.askSent)) {
        reading = text = "";
        await load();
      }
    });
  }

  async function step(a: AmendmentView, body: Record<string, unknown>, done: string) {
    await busy.run(async () => {
      message = null;
      const res = await call<{ errors?: string[] }>(
        "PUT",
        `/api/patients/${patientId}/amendments/${a.id}`,
        body,
        PURPOSE,
      );
      if (report(res, 200, done)) {
        stating = null;
        statement = "";
        await load();
      }
    });
  }

  async function disagree(event: SubmitEvent, a: AmendmentView) {
    event.preventDefault();
    const built = disagreeBody(a.version, statement);
    if (!built.ok) {
      message = built.problem;
      return;
    }
    await step(a, built.body, STRINGS.amendDone);
  }
</script>

<h3 id="ask-h-{patientId}">{STRINGS.askTitle}</h3>
{#if status === null}
  <p>{STRINGS.loading}</p>
{:else if status === 403}
  <p>{STRINGS.askNotAvailable}</p>
{:else if status !== 200}
  <p>{STRINGS.askCouldNot}</p>
{:else}
  <p>{STRINGS.askIntro}</p>
  {#if standing.length + standingAllergies.length === 0}
    <p>{STRINGS.askNoReadings}</p>
  {:else}
    <form onsubmit={ask}>
      <label for="ask-r-{patientId}">{STRINGS.askReading}</label>
      <select id="ask-r-{patientId}" bind:value={reading}>
        <option value=""></option>
        {#if standing.length}
          <optgroup label={STRINGS.askReadingGroup}>
            {#each standing as o (o.id)}
              <option value={`Observation:${o.id}`}>{vitalLine(o, siteZone)}</option>
            {/each}
          </optgroup>
        {/if}
        {#if standingAllergies.length}
          <optgroup label={STRINGS.askAllergyGroup}>
            {#each standingAllergies as g (g.id)}
              <option value={`AllergyIntolerance:${g.id}`}>{allergyLine(g)}</option>
            {/each}
          </optgroup>
        {/if}
      </select>
      <label for="ask-t-{patientId}">{STRINGS.askText}</label>
      <textarea id="ask-t-{patientId}" bind:value={text}></textarea>
      <button type="submit" disabled={busy.on}>{STRINGS.askSubmit}</button>
    </form>
  {/if}
  <h4>{STRINGS.askYours}</h4>
  {#if rows.length === 0}
    <p>{STRINGS.askNone}</p>
  {:else}
    <ul>
      {#each rows as a (a.id)}
        {@const s = a.request.state}
        <li>
          <strong>{STRINGS.askAboutReading.replace("{reading}", aboutText(a, lines))}</strong>
          · {siteWhenText(a.request.received_at, siteZone)}
          <p>{a.request.text}</p>
          <p>{stateText(s)} {dueText(a, siteZone)}</p>
          {#if s.state === "denied"}
            <p>{STRINGS.askReasonGiven} {s.reason}</p>
            {#if s.disagreement}<p>{STRINGS.askYourStatement} {s.disagreement}</p>{/if}
          {/if}
          {#each ownStepsFor(a) as st (st)}
            {#if st === "withdraw"}
              <button
                type="button"
                disabled={busy.on}
                onclick={() => step(a, { expected_version: a.version, step: "withdraw" }, STRINGS.askWithdrawn)}
              >
                {STRINGS.askWithdraw}
              </button>
            {:else if stating === a.id}
              <form onsubmit={(e) => disagree(e, a)}>
                <label for="ask-s-{a.id}">{STRINGS.askDisagreeLabel}</label>
                <textarea id="ask-s-{a.id}" bind:value={statement}></textarea>
                <button type="submit" disabled={busy.on}>{STRINGS.askDisagreeSend}</button>
                <button type="button" onclick={() => (stating = null)}>{STRINGS.amendCancel}</button>
              </form>
            {:else}
              <button type="button" onclick={() => ((stating = a.id), (statement = ""), (message = null))}>
                {STRINGS.askDisagree}
              </button>
            {/if}
          {/each}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
{#if message}<p role="status">{message}</p>{/if}
