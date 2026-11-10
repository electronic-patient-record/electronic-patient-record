<script lang="ts">
  // Record a suspicion's outcome (RS-3, RS-8), against the version on screen, or mark the recorded
  // outcome entered in error (RS-7). Nothing is preselected; each kind asks only for what it carries.
  // A 409 means the suspicion changed: the view reloads. The service's refusals are shown as it words them.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import {
    EMPTY_OUTCOME,
    buildOutcome,
    type OutcomeForm,
  } from "./suspicions.js";
  import { TEXT } from "./i18n.js";

  let {
    patientId,
    referralId,
    version,
    outcomeSeq,
    done,
  }: {
    patientId: string;
    referralId: string;
    version: number;
    outcomeSeq: number | null;
    done: () => void;
  } = $props();

  const busy = new Busy();
  let f = $state<OutcomeForm>({ ...EMPTY_OUTCOME });
  let reason = $state("");
  let message = $state<string | null>(null);
  const id = (name: string) => `end-${name}-${referralId}`;
  const base = $derived(
    `/api/patients/${patientId}/referrals/${referralId}/suspicion/outcome`,
  );

  async function settle(
    res: { status: number; body: unknown },
    ok: string,
    okStatus: number,
  ) {
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = TEXT[common];
    else if (res.status === okStatus) {
      message = ok;
      done();
    } else if (res.status === 409) {
      message = TEXT.referralChanged;
      done();
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = TEXT.error;
  }

  async function end(event: SubmitEvent) {
    event.preventDefault();
    const built = buildOutcome(f, version);
    if (built.problems.length > 0) {
      message = TEXT.endMissing.replace(
        "{fields}",
        built.problems.map((p) => TEXT.endProblems[p]).join(", "),
      );
      return;
    }
    await busy.run(async () => {
      message = null;
      await settle(await call("POST", base, built.body), TEXT.endDone, 201);
    });
  }

  async function markError(event: SubmitEvent) {
    event.preventDefault();
    if (!reason.trim() || outcomeSeq === null) {
      message = TEXT.errorMissing;
      return;
    }
    await busy.run(async () => {
      message = null;
      await settle(
        await call("POST", `${base}/entered-in-error`, {
          seq: outcomeSeq,
          reason: reason.trim(),
        }),
        TEXT.errorDone,
        200,
      );
    });
  }
</script>

{#if outcomeSeq === null}
  <details>
    <summary>{TEXT.endTitle}</summary>
    <p>{TEXT.endNote}</p>
    <form onsubmit={end} novalidate>
      <label for={id("kind")}>{TEXT.endKind}</label>
      <select id={id("kind")} bind:value={f.kind}>
        <option value="">{TEXT.raiseChoose}</option>
        {#each Object.entries(TEXT.endKinds) as [v, words] (v)}
          <option value={v}>{words}</option>
        {/each}
      </select>
      {#if f.kind === "confirmed"}
        <label for={id("diagnosis")}>{TEXT.endDiagnosis}</label>
        <input
          id={id("diagnosis")}
          bind:value={f.diagnosis}
          maxlength="200"
          autocomplete="off"
        />
      {/if}
      {#if f.kind === "ruled-out"}
        <label for={id("by")}>{TEXT.endBy}</label>
        <input
          id={id("by")}
          bind:value={f.by}
          maxlength="200"
          autocomplete="off"
        />
      {/if}
      <label for={id("detail")}>{TEXT.endDetail}</label>
      <input
        id={id("detail")}
        bind:value={f.detail}
        maxlength="500"
        autocomplete="off"
      />
      <button disabled={busy.on} type="submit">{TEXT.endSend}</button>
    </form>
  </details>
{:else}
  <details>
    <summary>{TEXT.errorTitle}</summary>
    <p>{TEXT.errorNote}</p>
    <form onsubmit={markError} novalidate>
      <label for={id("reason")}>{TEXT.errorReason}</label>
      <input
        id={id("reason")}
        bind:value={reason}
        maxlength="500"
        autocomplete="off"
      />
      <button disabled={busy.on} type="submit">{TEXT.errorSend}</button>
    </form>
  </details>
{/if}
{#if message}<p role="status">{message}</p>{/if}
