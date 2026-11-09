<script lang="ts">
  // What a person needs in order to use care (PA-6, H-048): record one, check it with them, end it. The
  // service decides who may and refuses what it must; this says so in words. An ended need stays in the
  // history. Who acts is the token.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import {
    CATALOGUE,
    CATEGORIES,
    active,
    adjustmentDetail,
    adjustmentName,
    changeBody,
    emptyAdjustment,
    recordBody,
    type AdjustmentLoad,
    type StoredAdjustment,
  } from "./adjustments.js";
  import { TEXT } from "./i18n.js";

  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
    load,
    onchange,
    purpose,
    own = false,
    readOnly = false,
  }: {
    patientId: string;
    siteZone?: string;
    // What the page's adjustment call returned; the banner reads the same answer.
    load: AdjustmentLoad | null;
    onchange: () => void | Promise<void>;
    // The purpose of use, for a page that fixes it, such as "my record".
    purpose?: string;
    // The reader is the patient, so the words are to them.
    own?: boolean;
    // The reader may look and not change (a proxy).
    readOnly?: boolean;
  } = $props();

  const rows = $derived(load?.adjustments ?? []);
  const standing = $derived(active(rows));
  const ended = $derived(rows.filter((a) => a.adjustment.status === "ended"));

  let message = $state<string | null>(null);
  let form = $state(emptyAdjustment());
  let recording = $state(false);
  // The one change being filled in, on one adjustment.
  let open = $state<{ id: string; action: "review" | "end" } | null>(null);
  let reason = $state("");
  let reviewBy = $state("");

  $effect(() => {
    void patientId;
    open = null;
    recording = false;
    form = emptyAdjustment();
    message = null;
  });

  function failed(res: { status: number; body?: { errors?: string[] } }) {
    const common = commonProblem(res.status);
    if (common) return TEXT[common];
    if (res.status === 409) return TEXT.adjustmentConflict;
    if (res.status === 422 && res.body?.errors)
      return res.body.errors.join(" ");
    return TEXT.error;
  }

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(async () => {
      message = null;
      const built = recordBody(form);
      if (!built.ok) {
        message = built.problem;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "POST",
        `/api/patients/${patientId}/adjustments`,
        built.body,
        purpose,
      );
      if (res.status === 201) {
        message = TEXT.adjustmentSaved;
        recording = false;
        form = emptyAdjustment();
        await onchange();
      } else message = failed(res);
    });
  }

  async function change(event: SubmitEvent, a: StoredAdjustment) {
    event.preventDefault();
    if (!open) return;
    const action = open.action;
    await busy.run(async () => {
      message = null;
      const built = changeBody(action, a.version, reviewBy, reason);
      if (!built.ok) {
        message = built.problem;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "PUT",
        `/api/patients/${patientId}/adjustments/${a.id}`,
        built.body,
        purpose,
      );
      if (res.status === 200) {
        message = TEXT.adjustmentSaved;
        open = null;
        await onchange();
      } else message = failed(res);
    });
  }

  function start(id: string, action: "review" | "end") {
    open = { id, action };
    reason = "";
    reviewBy = "";
    message = null;
  }
</script>

<section aria-labelledby="adjustments-h">
  <h2 id="adjustments-h">{TEXT.adjustmentTitle}</h2>
  <p>{own ? TEXT.adjustmentHelpOwn : TEXT.adjustmentHelp}</p>
  {#if load === null}
    <p>{TEXT.loading}</p>
  {:else if load.status === 403}
    <p>{TEXT.bannerNeedsNotShown}</p>
  {:else if load.status !== 200}
    <p>{TEXT.bannerNeedsFailed}</p>
  {:else if standing.length === 0}
    <p>{TEXT.bannerNeedsNone}</p>
  {:else}
    <ul>
      {#each standing as a (a.id)}
        <li>
          <strong>{adjustmentName(a)}</strong>
          <p>{adjustmentDetail(a, siteZone)}</p>
          <p>{TEXT.adjustmentReasonOf} {a.adjustment.reason}</p>
          {#if readOnly}
            <!-- A proxy reads; a change is the patient's or a member of staff's. -->
          {:else if open?.id === a.id}
            <form onsubmit={(e) => change(e, a)}>
              {#if open.action === "review"}
                <label for="adjustment-next-{a.id}"
                  >{TEXT.adjustmentNextReview}</label
                >
                <input
                  id="adjustment-next-{a.id}"
                  type="date"
                  bind:value={reviewBy}
                />
              {/if}
              <label for="adjustment-why-{a.id}">{TEXT.adjustmentReason}</label>
              <input
                id="adjustment-why-{a.id}"
                bind:value={reason}
                autocomplete="off"
              />
              <button type="submit" disabled={busy.on}>
                {open.action === "review"
                  ? TEXT.adjustmentConfirmReview
                  : TEXT.adjustmentConfirmEnd}
              </button>
              <button type="button" onclick={() => (open = null)}>
                {TEXT.adjustmentCancel}
              </button>
            </form>
          {:else}
            <button type="button" onclick={() => start(a.id, "review")}>
              {TEXT.adjustmentReview}
            </button>
            <button type="button" onclick={() => start(a.id, "end")}>
              {TEXT.adjustmentEnd}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if ended.length}
    <details>
      <summary
        >{TEXT.adjustmentEndedTitle.replace(
          "{n}",
          String(ended.length),
        )}</summary
      >
      <ul>
        {#each ended as a (a.id)}
          <li>
            <s>{adjustmentName(a)}</s>
            · {adjustmentDetail(a, siteZone)}
            · {TEXT.adjustmentReasonOf}
            {a.adjustment.reason}
          </li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if load?.status === 200 && !readOnly}
    {#if recording}
      <form onsubmit={record}>
        <label for="adjustment-code">{TEXT.adjustmentNeed}</label>
        <select id="adjustment-code" bind:value={form.code}>
          <option value=""></option>
          {#each CATEGORIES as category (category)}
            <optgroup label={TEXT.adjustmentCategories[category]}>
              {#each CATALOGUE[category] as code (code)}
                <option value={code}>{TEXT.adjustmentCodes[code]}</option>
              {/each}
            </optgroup>
          {/each}
        </select>
        <label for="adjustment-note">{TEXT.adjustmentNote}</label>
        <input
          id="adjustment-note"
          bind:value={form.note}
          maxlength="200"
          autocomplete="off"
        />
        <label for="adjustment-review">{TEXT.adjustmentReviewDate}</label>
        <input id="adjustment-review" type="date" bind:value={form.reviewBy} />
        <label for="adjustment-reason">{TEXT.adjustmentReason}</label>
        <input
          id="adjustment-reason"
          bind:value={form.reason}
          autocomplete="off"
        />
        <button type="submit" disabled={busy.on}>
          {TEXT.adjustmentSave}
        </button>
        <button type="button" onclick={() => (recording = false)}>
          {TEXT.adjustmentCancel}
        </button>
      </form>
    {:else}
      <button type="button" onclick={() => (recording = true)}>
        {TEXT.adjustmentAdd}
      </button>
    {/if}
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
