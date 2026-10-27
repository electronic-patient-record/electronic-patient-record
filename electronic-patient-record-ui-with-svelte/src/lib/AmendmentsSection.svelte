<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // Requests to correct a record (RC-4, H-035): a clinician reads what the patient asked and agrees,
  // refuses (with a ground and a reason), records the correction, or takes the one extension. Who is
  // acting is the token. The patient's words are shown as text. Nothing is sent without its reason.
  import { call, commonProblem } from "./api.js";
  import { siteWhenText } from "./access-log.js";
  import { loadAllergies, type StoredAllergy } from "./allergies.js";
  import { loadObservations, type StoredObservation } from "./vitals.js";
  import {
    GROUNDS,
    aboutText,
    dueText,
    loadAmendments,
    openCounts,
    targetLines,
    stateText,
    stepBody,
    stepsFor,
    type AmendmentView,
    type Ground,
    type StepName,
  } from "./amendments.js";
  import { STRINGS } from "./strings.js";

  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
    refresh = null,
  }: { patientId: string; siteZone?: string; refresh?: unknown } = $props();

  let status = $state<number | null>(null);
  let rows = $state<AmendmentView[]>([]);
  let observations = $state<StoredObservation[]>([]);
  let allergies = $state<StoredAllergy[]>([]);
  const lines = $derived(targetLines(observations, allergies, siteZone));
  const counts = $derived(openCounts(rows));
  let message = $state<string | null>(null);
  // The one step being filled in, on one request.
  let open = $state<{ id: string; step: StepName } | null>(null);
  let reason = $state("");
  let ground = $state<Ground | "">("");
  let version = $state("");

  const label: Record<StepName, string> = {
    accept: STRINGS.amendAccept,
    deny: STRINGS.amendDeny,
    apply: STRINGS.amendApply,
    extend: STRINGS.amendExtend,
  };

  // A late answer for the patient the page has moved away from is dropped.
  let ticket = 0;
  async function load() {
    const mine = ++ticket;
    const forId = patientId;
    const [res, obs, alg] = await Promise.all([
      loadAmendments(forId),
      loadObservations(forId),
      loadAllergies(forId),
    ]);
    if (mine !== ticket || forId !== patientId) return;
    status = res.status;
    rows = res.amendments ?? [];
    // A record this role cannot read is not named; the request still shows its kind.
    observations = obs.observations ?? [];
    allergies = alg.data?.allergies ?? [];
  }
  $effect(() => {
    void patientId;
    void refresh;
    status = null;
    rows = [];
    observations = [];
    allergies = [];
    open = null;
    void load();
  });

  function start(a: AmendmentView, step: StepName) {
    open = { id: a.id, step };
    reason = "";
    ground = "";
    version = "";
    message = null;
  }

  async function send(event: SubmitEvent, a: AmendmentView) {
    event.preventDefault();
    if (!open) return;
    const step = open.step;
    await busy.run(async () => {
      message = null;
      const built = stepBody(step, a.version, { reason, ground, version });
      if (!built.ok) {
        message = built.problem;
        return;
      }
      const res = await call<{ errors?: string[] }>(
        "PUT",
        `/api/patients/${patientId}/amendments/${a.id}`,
        built.body,
      );
      const common = commonProblem(res.status);
      if (common) message = STRINGS[common];
      else if (res.status === 200) {
        message = STRINGS.amendDone;
        open = null;
        await load();
      } else if (res.status === 409) message = STRINGS.amendConflict;
      else if (res.status === 422 && res.body?.errors) message = res.body.errors.join(" ");
      else message = STRINGS.error;
    });
  }
</script>

<section aria-labelledby="amend-h">
  <h2 id="amend-h">{STRINGS.amendTitle}</h2>
  {#if status === null}
    <p>{STRINGS.loading}</p>
  {:else if status === 403}
    <p>{STRINGS.amendNotShown}</p>
  {:else if status !== 200}
    <p>{STRINGS.amendCouldNot}</p>
  {:else if rows.length === 0}
    <p>{STRINGS.amendNone}</p>
  {:else}
    {#if counts.open > 0}
      <p role="status">
        {STRINGS.amendOpenCount.replace("{n}", String(counts.open))}
        {#if counts.overdue > 0}<strong>{STRINGS.amendOverdueCount.replace("{n}", String(counts.overdue))}</strong>{/if}
      </p>
    {/if}
    <ul>
      {#each rows as a (a.id)}
        {@const s = a.request.state}
        <li>
          <strong>{STRINGS.amendAbout.replace("{kind}", aboutText(a, lines))}</strong>
          · {STRINGS.amendAsked.replace("{when}", siteWhenText(a.request.received_at, siteZone))}
          <p>{STRINGS.amendText} {a.request.text}</p>
          <p>{stateText(s)} {dueText(a, siteZone)}</p>
          {#if s.state === "accepted" || s.state === "denied"}
            <p>{STRINGS.amendReasonOf} {s.reason}</p>
          {/if}
          {#if s.state === "denied" && s.disagreement}
            <p>{STRINGS.noticeStatement} {s.disagreement}</p>
          {/if}
          {#if a.request.extension}<p>{STRINGS.amendExtended} {a.request.extension}</p>{/if}
          {#if open?.id === a.id}
            <form onsubmit={(e) => send(e, a)}>
              <p>{label[open.step]}</p>
              {#if open.step === "deny"}
                <label for="am-g-{a.id}">{STRINGS.amendGround}</label>
                <select id="am-g-{a.id}" bind:value={ground}>
                  <option value=""></option>
                  {#each GROUNDS as g (g)}
                    <option value={g}>{STRINGS.amendGroundLabels[g]}</option>
                  {/each}
                </select>
              {/if}
              {#if open.step === "apply"}
                <label for="am-v-{a.id}">{STRINGS.amendVersion}</label>
                <input id="am-v-{a.id}" bind:value={version} inputmode="numeric" autocomplete="off" />
              {:else}
                <label for="am-r-{a.id}">{STRINGS.amendReason}</label>
                <input id="am-r-{a.id}" bind:value={reason} autocomplete="off" />
              {/if}
              <button type="submit" disabled={busy.on}>{STRINGS.amendConfirm}</button>
              <button type="button" onclick={() => (open = null)}>{STRINGS.amendCancel}</button>
            </form>
          {:else}
            {#each stepsFor(a) as step (step)}
              <button type="button" onclick={() => start(a, step)}>{label[step]}</button>
            {/each}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
