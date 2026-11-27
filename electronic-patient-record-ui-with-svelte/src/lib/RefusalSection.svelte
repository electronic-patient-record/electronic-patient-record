<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // Record a refusal of treatment (RF-7, "recording one"; UNREVIEWED wording). It starts with the person's own words; every
  // yes-or-no allows "not known", and one button marks each question not yet answered as not known, so a form is never
  // stuck and nothing is guessed. Nothing here says whether the refusal is valid or whether the person may be treated.
  // Shown only to a role that may read refusals; the API refuses one that may not record.
  import { call, commonProblem } from "./api.js";
  import { newIdempotencyKey } from "./idempotency.js";
  import type { Kind, Known, SaidBy } from "./treatment-refusals.js";
  import { TEXT } from "./i18n.js";

  const busy = new Busy();
  let {
    patientId,
    onchange,
  }: {
    patientId: string;
    // Called after a refusal is recorded, so the page reloads the banner.
    onchange: () => void | Promise<void>;
  } = $props();

  const KINDS: Kind[] = [
    "spoken-now",
    "advance",
    "order",
    "representative",
    "parent",
    "carried",
    "reported",
  ];
  const SAID: SaidBy[] = [
    "patient",
    "attorney",
    "guardian",
    "parent",
    "relative",
    "clinician",
    "other",
  ];
  const KNOWN: Known[] = ["yes", "no", "not-known"];

  let treatment = $state("");
  let accepts = $state("");
  let circumstances = $state("");
  let kind = $state<Kind | "">("");
  let saidBy = $state<SaidBy | "">("");
  let powerSeen = $state<Known | "">("");
  let lifeAtRisk = $state<"yes" | "no" | "not-stated" | "">("");
  let written = $state<Known | "">("");
  let signed = $state<Known | "">("");
  let witnessed = $state<Known | "">("");
  let seen = $state<"original" | "copy" | "not-seen" | "">("");
  let kept = $state("");
  let madeOn = $state("");
  let confirmedOn = $state("");
  let uncertain = $state(false);
  let message = $state<string | null>(null);
  // One key per refusal; a new one after each success (OF-1).
  let idempotencyKey = newIdempotencyKey();

  const needsPower = $derived(kind === "representative" || kind === "parent");

  function notKnownForTheRest() {
    if (lifeAtRisk === "") lifeAtRisk = "not-stated";
    if (written === "") written = "not-known";
    if (signed === "") signed = "not-known";
    if (witnessed === "") witnessed = "not-known";
    if (seen === "") seen = "not-seen";
    if (needsPower && powerSeen === "") powerSeen = "not-known";
    if (saidBy === "") saidBy = "other";
  }

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => recordNow());
  }

  async function recordNow() {
    message = null;
    const unanswered = [
      [treatment.trim(), TEXT.rfWhat],
      [kind, TEXT.rfKind],
      [saidBy, TEXT.rfSaidBy],
      [lifeAtRisk, TEXT.emLifeAtRisk],
      [written, TEXT.emWritten],
      [signed, TEXT.emSigned],
      [witnessed, TEXT.emWitnessed],
      [seen, TEXT.emSeen],
      ...(needsPower ? [[powerSeen, TEXT.emPowerSeen]] : []),
    ]
      .filter(([v]) => v === "")
      .map(([, label]) => label);
    if (unanswered.length > 0) {
      message = TEXT.rfUnanswered.replace("{fields}", unanswered.join(", "));
      return;
    }
    const refusal = {
      kind,
      treatment: treatment.trim(),
      ...(accepts.trim() ? { accepts: accepts.trim() } : {}),
      ...(circumstances.trim() ? { circumstances: circumstances.trim() } : {}),
      if_life_at_risk: lifeAtRisk,
      form: {
        written,
        signed,
        witnessed,
        seen,
        ...(kept.trim() ? { kept: kept.trim() } : {}),
      },
      ...(madeOn.trim() ? { made_on: madeOn.trim() } : {}),
      ...(confirmedOn.trim() ? { confirmed_on: confirmedOn.trim() } : {}),
      said_by: saidBy,
      ...(needsPower ? { power_seen: powerSeen } : {}),
      identity: uncertain ? "uncertain" : "certain",
      state: "stands",
    };
    const res = await call(
      "POST",
      `/api/patients/${patientId}/treatment-refusals`,
      { refusal },
      undefined,
      { idempotencyKey },
    );
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = TEXT[common];
    else if (res.status === 201) {
      treatment = accepts = circumstances = kept = madeOn = confirmedOn = "";
      kind =
        saidBy =
        powerSeen =
        lifeAtRisk =
        written =
        signed =
        witnessed =
        seen =
          "";
      uncertain = false;
      idempotencyKey = newIdempotencyKey();
      message = TEXT.rfSaved;
      await onchange();
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = TEXT.error;
  }
</script>

<section aria-labelledby="refusals-h">
  <h2 id="refusals-h">{TEXT.rfTitle}</h2>
  <p role="note">{TEXT.rfIntro}</p>
  <form onsubmit={record}>
    <label for="rf-what">{TEXT.rfWhat}</label>
    <input id="rf-what" bind:value={treatment} autocomplete="off" />
    <label for="rf-accepts">{TEXT.emAccepts}</label>
    <input id="rf-accepts" bind:value={accepts} autocomplete="off" />
    <label for="rf-when">{TEXT.emCircumstances}</label>
    <input id="rf-when" bind:value={circumstances} autocomplete="off" />
    <label for="rf-kind">{TEXT.rfKind}</label>
    <select id="rf-kind" bind:value={kind}>
      <option value="">{TEXT.rfChoose}</option>
      {#each KINDS as k (k)}<option value={k}>
          {(TEXT.emKinds as Record<string, string>)[k]}
        </option>{/each}
    </select>
    <label for="rf-said">{TEXT.rfSaidBy}</label>
    <select id="rf-said" bind:value={saidBy}>
      <option value="">{TEXT.rfChoose}</option>
      {#each SAID as s (s)}<option value={s}>
          {(TEXT.emSaidByWords as Record<string, string>)[s]}
        </option>{/each}
    </select>
    {#if needsPower}
      <label for="rf-power">{TEXT.emPowerSeen}</label>
      <select id="rf-power" bind:value={powerSeen}>
        <option value="">{TEXT.rfChoose}</option>
        {#each KNOWN as k (k)}<option value={k}>
            {(TEXT.emKnown as Record<string, string>)[k]}
          </option>{/each}
      </select>
    {/if}
    <label for="rf-life">{TEXT.emLifeAtRisk}</label>
    <select id="rf-life" bind:value={lifeAtRisk}>
      <option value="">{TEXT.rfChoose}</option>
      {#each ["yes", "no", "not-stated"] as l (l)}<option value={l}>
          {(TEXT.emLifeWords as Record<string, string>)[l]}
        </option>{/each}
    </select>
    {#each [["rf-written", TEXT.emWritten], ["rf-signed", TEXT.emSigned], ["rf-witnessed", TEXT.emWitnessed]] as [fid, label] (fid)}
      <label for={fid}>{label}</label>
      {#if fid === "rf-written"}
        <select id={fid} bind:value={written}>
          <option value="">{TEXT.rfChoose}</option>
          {#each KNOWN as k (k)}<option value={k}>
              {(TEXT.emKnown as Record<string, string>)[k]}
            </option>{/each}
        </select>
      {:else if fid === "rf-signed"}
        <select id={fid} bind:value={signed}>
          <option value="">{TEXT.rfChoose}</option>
          {#each KNOWN as k (k)}<option value={k}>
              {(TEXT.emKnown as Record<string, string>)[k]}
            </option>{/each}
        </select>
      {:else}
        <select id={fid} bind:value={witnessed}>
          <option value="">{TEXT.rfChoose}</option>
          {#each KNOWN as k (k)}<option value={k}>
              {(TEXT.emKnown as Record<string, string>)[k]}
            </option>{/each}
        </select>
      {/if}
    {/each}
    <label for="rf-seen">{TEXT.emSeen}</label>
    <select id="rf-seen" bind:value={seen}>
      <option value="">{TEXT.rfChoose}</option>
      {#each ["original", "copy", "not-seen"] as s (s)}<option value={s}>
          {(TEXT.emSeenWords as Record<string, string>)[s]}
        </option>{/each}
    </select>
    <label for="rf-kept">{TEXT.emKept}</label>
    <input id="rf-kept" bind:value={kept} autocomplete="off" />
    <label for="rf-made">{TEXT.rfMadeOn}</label>
    <input id="rf-made" bind:value={madeOn} autocomplete="off" />
    <label for="rf-confirmed">{TEXT.rfConfirmedOn}</label>
    <input id="rf-confirmed" bind:value={confirmedOn} autocomplete="off" />
    <label>
      <input type="checkbox" bind:checked={uncertain} />
      {TEXT.rfUncertain}
    </label>
    <button type="button" onclick={notKnownForTheRest} disabled={busy.on}>
      {TEXT.rfNotKnownRest}
    </button>
    <button type="submit" disabled={busy.on}>{TEXT.rfRecord}</button>
  </form>
  {#if message}<p role="status">{message}</p>{/if}
</section>
