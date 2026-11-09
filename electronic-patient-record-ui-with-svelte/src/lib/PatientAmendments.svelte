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
  import {
    allergyLine,
    loadAllergies,
    type StoredAllergy,
  } from "./allergies.js";
  import { TEXT } from "./i18n.js";
  import {
    loadObservations,
    vitalLine,
    type StoredObservation,
  } from "./vitals.js";

  const PURPOSE = "patient-access";
  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
  }: { patientId: string; siteZone?: string } = $props();

  let status = $state<number | null>(null);
  let rows = $state<AmendmentView[]>([]);
  let readings = $state<StoredObservation[] | null>(null);
  let allergies = $state<StoredAllergy[] | null>(null);
  let message = $state<string | null>(null);

  let reading = $state("");
  let text = $state("");
  let stating = $state<string | null>(null);
  let statement = $state("");

  const standing = $derived(
    (readings ?? []).filter((o) => o.status !== "entered-in-error"),
  );
  const standingAllergies = $derived(
    (allergies ?? []).filter((a) => a.allergy.status !== "entered-in-error"),
  );
  const lines = $derived(
    targetLines(readings ?? [], allergies ?? [], siteZone),
  );

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

  function report(
    res: { status: number; body: { errors?: string[] } | null },
    ok: number,
    done: string,
  ) {
    const common = commonProblem(res.status);
    if (common) message = TEXT[common];
    else if (res.status === ok) message = done;
    else if (res.status === 409) message = TEXT.amendConflict;
    else if (res.status === 422 && res.body?.errors)
      message = res.body.errors.join(" ");
    else message = TEXT.error;
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
      const built = askBody(
        found ? { kind, id: found.id, version: found.version } : null,
        text,
      );
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
      if (report(res, 201, TEXT.askSent)) {
        reading = text = "";
        await load();
      }
    });
  }

  async function step(
    a: AmendmentView,
    body: Record<string, unknown>,
    done: string,
  ) {
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
    await step(a, built.body, TEXT.amendDone);
  }
</script>

<h3 id="ask-h-{patientId}">{TEXT.askTitle}</h3>
{#if status === null}
  <p>{TEXT.loading}</p>
{:else if status === 403}
  <p>{TEXT.askNotAvailable}</p>
{:else if status !== 200}
  <p>{TEXT.askCouldNot}</p>
{:else}
  <p>{TEXT.askIntro}</p>
  {#if standing.length + standingAllergies.length === 0}
    <p>{TEXT.askNoReadings}</p>
  {:else}
    <form onsubmit={ask}>
      <label for="ask-r-{patientId}">{TEXT.askReading}</label>
      <select id="ask-r-{patientId}" bind:value={reading}>
        <option value=""></option>
        {#if standing.length}
          <optgroup label={TEXT.askReadingGroup}>
            {#each standing as o (o.id)}
              <option value={`Observation:${o.id}`}
                >{vitalLine(o, siteZone)}</option
              >
            {/each}
          </optgroup>
        {/if}
        {#if standingAllergies.length}
          <optgroup label={TEXT.askAllergyGroup}>
            {#each standingAllergies as g (g.id)}
              <option value={`AllergyIntolerance:${g.id}`}
                >{allergyLine(g)}</option
              >
            {/each}
          </optgroup>
        {/if}
      </select>
      <label for="ask-t-{patientId}">{TEXT.askText}</label>
      <textarea id="ask-t-{patientId}" bind:value={text}></textarea>
      <button type="submit" disabled={busy.on}>{TEXT.askSubmit}</button>
    </form>
  {/if}
  <h4>{TEXT.askYours}</h4>
  {#if rows.length === 0}
    <p>{TEXT.askNone}</p>
  {:else}
    <ul>
      {#each rows as a (a.id)}
        {@const s = a.request.state}
        <li>
          <strong
            >{TEXT.askAboutReading.replace(
              "{reading}",
              aboutText(a, lines),
            )}</strong
          >
          · {siteWhenText(a.request.received_at, siteZone)}
          <p>{a.request.text}</p>
          <p>{stateText(s)} {dueText(a, siteZone)}</p>
          {#if s.state === "denied"}
            <p>{TEXT.askReasonGiven} {s.reason}</p>
            {#if s.disagreement}<p>
                {TEXT.askYourStatement}
                {s.disagreement}
              </p>{/if}
          {/if}
          {#each ownStepsFor(a) as st (st)}
            {#if st === "withdraw"}
              <button
                type="button"
                disabled={busy.on}
                onclick={() =>
                  step(
                    a,
                    { expected_version: a.version, step: "withdraw" },
                    TEXT.askWithdrawn,
                  )}
              >
                {TEXT.askWithdraw}
              </button>
            {:else if stating === a.id}
              <form onsubmit={(e) => disagree(e, a)}>
                <label for="ask-s-{a.id}">{TEXT.askDisagreeLabel}</label>
                <textarea id="ask-s-{a.id}" bind:value={statement}></textarea>
                <button type="submit" disabled={busy.on}
                  >{TEXT.askDisagreeSend}</button
                >
                <button type="button" onclick={() => (stating = null)}
                  >{TEXT.amendCancel}</button
                >
              </form>
            {:else}
              <button
                type="button"
                onclick={() => (
                  (stating = a.id),
                  (statement = ""),
                  (message = null)
                )}
              >
                {TEXT.askDisagree}
              </button>
            {/if}
          {/each}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
{#if message}<p role="status">{message}</p>{/if}
