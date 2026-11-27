<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // Record a refusal of treatment (RF-7, "recording one"; UNREVIEWED wording). It starts with the person's own words; every
  // yes-or-no allows "not known", and one button marks each question not yet answered as not known, so a form is never
  // stuck and nothing is guessed. Nothing here says whether the refusal is valid or whether the person may be treated.
  // Shown only to a role that may read refusals; the API refuses one that may not record.
  import { call, commonProblem } from "./api.js";
  import { newIdempotencyKey } from "./idempotency.js";
  import type {
    Kind,
    Known,
    ListedRefusal,
    RefusalData,
    SaidBy,
  } from "./treatment-refusals.js";
  import { TEXT } from "./i18n.js";

  const busy = new Busy();
  let {
    patientId,
    data,
    onchange,
  }: {
    patientId: string;
    // The refusals on record, as the banner read them.
    data: RefusalData;
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

  // A change to one on record (a doubt, a withdrawal heard, entered in error): always a new version, with why.
  const open = $derived(
    data.refusals.filter(
      (r) => r.refusal.state === "stands" || r.refusal.state === "doubted",
    ),
  );
  let changing = $state<string | null>(null);
  let action = $state<"doubt" | "withdrawal" | "error" | "stands">("doubt");
  let why = $state("");
  let how = $state<"spoken" | "written" | "conduct">("spoken");
  let noticedBy = $state("");
  let heardAt = $state("");

  function startChange(r: ListedRefusal, a: typeof action) {
    changing = r.id;
    action = a;
    why = a === "withdrawal" ? TEXT.rfWithdrawalWhy : "";
    noticedBy = heardAt = "";
    message = null;
  }

  async function change(event: SubmitEvent, r: ListedRefusal) {
    event.preventDefault();
    await busy.run(() => changeNow(r));
  }

  async function changeNow(r: ListedRefusal) {
    message = null;
    if (!why.trim()) {
      message = TEXT.rfWhyNeeded;
      return;
    }
    const base = r.refusal as Record<string, unknown>;
    let next: Record<string, unknown>;
    if (action === "doubt") {
      next = { ...base, state: "doubted", doubt: why.trim() };
    } else if (action === "withdrawal") {
      const at = heardAt ? new Date(heardAt) : null;
      if (!noticedBy.trim() || !at || Number.isNaN(at.getTime())) {
        message = TEXT.rfWithdrawalNeeded;
        return;
      }
      // Kept as heard; no rule in force says it ends the refusal, so it is doubted (RF-5).
      next = {
        ...base,
        state: "doubted",
        doubt: why.trim(),
        withdrawal: { how, noticed_by: noticedBy.trim(), at: at.toISOString() },
      };
    } else if (action === "stands") {
      // The doubt is resolved: it stands as recorded, and the doubt is kept in the history, not on the record.
      next = { ...base, state: "stands" };
      delete next.doubt;
    } else {
      next = { ...base, state: "entered-in-error" };
    }
    const res = await call(
      "PUT",
      `/api/patients/${patientId}/treatment-refusals/${r.id}`,
      { refusal: next, expected_version: r.version ?? 1, reason: why.trim() },
    );
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = TEXT[common];
    else if (res.status === 200) {
      changing = null;
      message = TEXT.rfSaved;
      await onchange();
    } else if (res.status === 409) {
      changing = null;
      message = TEXT.rfChanged;
      await onchange();
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = TEXT.error;
  }

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
  {#if open.length > 0}
    <h3>{TEXT.rfOnRecord}</h3>
    <ul>
      {#each open as r (r.id)}
        <li>
          <strong>{r.refusal.treatment}</strong>
          · {(TEXT.emStates as Record<string, string>)[r.refusal.state]}
          {#if r.from_merged_record}· {TEXT.refusalFromMerged}{/if}
          <button type="button" onclick={() => startChange(r, "doubt")}>
            {TEXT.rfDoubt}
          </button>
          <button type="button" onclick={() => startChange(r, "withdrawal")}>
            {TEXT.rfHeard}
          </button>
          {#if r.refusal.state === "doubted" && !r.refusal.withdrawal}
            <button type="button" onclick={() => startChange(r, "stands")}>
              {TEXT.rfStands}
            </button>
          {:else if r.refusal.state === "doubted"}
            <span>{TEXT.rfHeardNote}</span>
          {/if}
          <button type="button" onclick={() => startChange(r, "error")}>
            {TEXT.rfError}
          </button>
          {#if changing === r.id}
            <form onsubmit={(e) => change(e, r)}>
              {#if action === "withdrawal"}
                <label for="rf-how">{TEXT.rfHow}</label>
                <select id="rf-how" bind:value={how}>
                  {#each ["spoken", "written", "conduct"] as h (h)}<option
                      value={h}
                    >
                      {(TEXT.emHow as Record<string, string>)[h]}
                    </option>{/each}
                </select>
                <label for="rf-noticed">{TEXT.rfNoticedBy}</label>
                <input
                  id="rf-noticed"
                  bind:value={noticedBy}
                  autocomplete="off"
                />
                <label for="rf-at">{TEXT.rfHeardAt}</label>
                <input id="rf-at" type="datetime-local" bind:value={heardAt} />
              {/if}
              <label for="rf-why">
                {action === "error"
                  ? TEXT.rfErrorWhy
                  : action === "stands"
                    ? TEXT.rfStandsWhy
                    : TEXT.rfDoubtWhy}
              </label>
              <input id="rf-why" bind:value={why} autocomplete="off" />
              <button type="submit" disabled={busy.on}>
                {TEXT.rfSaveChange}
              </button>
              <button type="button" onclick={() => (changing = null)}>
                {TEXT.rfCancel}
              </button>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <h3>{TEXT.rfNew}</h3>
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
