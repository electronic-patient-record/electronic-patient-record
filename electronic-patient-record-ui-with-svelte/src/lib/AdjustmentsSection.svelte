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
  import { STRINGS } from "./strings.js";

  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
    load,
    onchange,
  }: {
    patientId: string;
    siteZone?: string;
    // What the page's adjustment call returned; the banner reads the same answer.
    load: AdjustmentLoad | null;
    onchange: () => void | Promise<void>;
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
    if (common) return STRINGS[common];
    if (res.status === 409) return STRINGS.adjustmentConflict;
    if (res.status === 422 && res.body?.errors)
      return res.body.errors.join(" ");
    return STRINGS.error;
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
      );
      if (res.status === 201) {
        message = STRINGS.adjustmentSaved;
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
      );
      if (res.status === 200) {
        message = STRINGS.adjustmentSaved;
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
  <h2 id="adjustments-h">{STRINGS.adjustmentTitle}</h2>
  <p>{STRINGS.adjustmentHelp}</p>
  {#if load === null}
    <p>{STRINGS.loading}</p>
  {:else if load.status === 403}
    <p>{STRINGS.bannerNeedsNotShown}</p>
  {:else if load.status !== 200}
    <p>{STRINGS.bannerNeedsFailed}</p>
  {:else if standing.length === 0}
    <p>{STRINGS.bannerNeedsNone}</p>
  {:else}
    <ul>
      {#each standing as a (a.id)}
        <li>
          <strong>{adjustmentName(a)}</strong>
          <p>{adjustmentDetail(a, siteZone)}</p>
          <p>{STRINGS.adjustmentReasonOf} {a.adjustment.reason}</p>
          {#if open?.id === a.id}
            <form onsubmit={(e) => change(e, a)}>
              {#if open.action === "review"}
                <label for="adjustment-next-{a.id}"
                  >{STRINGS.adjustmentNextReview}</label
                >
                <input
                  id="adjustment-next-{a.id}"
                  type="date"
                  bind:value={reviewBy}
                />
              {/if}
              <label for="adjustment-why-{a.id}"
                >{STRINGS.adjustmentReason}</label
              >
              <input
                id="adjustment-why-{a.id}"
                bind:value={reason}
                autocomplete="off"
              />
              <button type="submit" disabled={busy.on}>
                {open.action === "review"
                  ? STRINGS.adjustmentConfirmReview
                  : STRINGS.adjustmentConfirmEnd}
              </button>
              <button type="button" onclick={() => (open = null)}>
                {STRINGS.adjustmentCancel}
              </button>
            </form>
          {:else}
            <button type="button" onclick={() => start(a.id, "review")}>
              {STRINGS.adjustmentReview}
            </button>
            <button type="button" onclick={() => start(a.id, "end")}>
              {STRINGS.adjustmentEnd}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if ended.length}
    <details>
      <summary
        >{STRINGS.adjustmentEndedTitle.replace(
          "{n}",
          String(ended.length),
        )}</summary
      >
      <ul>
        {#each ended as a (a.id)}
          <li>
            <s>{adjustmentName(a)}</s>
            · {adjustmentDetail(a, siteZone)}
            · {STRINGS.adjustmentReasonOf}
            {a.adjustment.reason}
          </li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if load?.status === 200}
    {#if recording}
      <form onsubmit={record}>
        <label for="adjustment-code">{STRINGS.adjustmentNeed}</label>
        <select id="adjustment-code" bind:value={form.code}>
          <option value=""></option>
          {#each CATEGORIES as category (category)}
            <optgroup label={STRINGS.adjustmentCategories[category]}>
              {#each CATALOGUE[category] as code (code)}
                <option value={code}>{STRINGS.adjustmentCodes[code]}</option>
              {/each}
            </optgroup>
          {/each}
        </select>
        <label for="adjustment-note">{STRINGS.adjustmentNote}</label>
        <input
          id="adjustment-note"
          bind:value={form.note}
          maxlength="200"
          autocomplete="off"
        />
        <label for="adjustment-review">{STRINGS.adjustmentReviewDate}</label>
        <input id="adjustment-review" type="date" bind:value={form.reviewBy} />
        <label for="adjustment-reason">{STRINGS.adjustmentReason}</label>
        <input
          id="adjustment-reason"
          bind:value={form.reason}
          autocomplete="off"
        />
        <button type="submit" disabled={busy.on}>
          {STRINGS.adjustmentSave}
        </button>
        <button type="button" onclick={() => (recording = false)}>
          {STRINGS.adjustmentCancel}
        </button>
      </form>
    {:else}
      <button type="button" onclick={() => (recording = true)}>
        {STRINGS.adjustmentAdd}
      </button>
    {/if}
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
