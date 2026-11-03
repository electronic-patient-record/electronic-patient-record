<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  // Referrals (CA-3): the list, the form to send one, and the steps the service says may follow. The
  // service decides who may take a step (the sender of a referral is the referrer, anyone else with
  // access the recipient), so a button for the other party's step is answered with a refusal, not hidden.
  // A cancellation is a request until the recipient confirms it. Text from the referral is shown as text.
  import { call, commonProblem } from "./api.js";
  import { newIdempotencyKey } from "./idempotency.js";
  import { siteWhenText } from "./access-log.js";
  import {
    PRIORITIES,
    PROVISIONAL,
    loadReferrals,
    type NextEvent,
    type Priority,
    type StoredReferral,
  } from "./referrals.js";
  import { STRINGS } from "./strings.js";

  const busy = new Busy();
  let {
    patientId,
    siteZone = "UTC",
    refresh = null,
  }: { patientId: string; siteZone?: string; refresh?: unknown } = $props();

  let status = $state<number | null>(null);
  let rows = $state<StoredReferral[]>([]);
  let message = $state<string | null>(null);
  let recipient = $state("");
  let reason = $state("");
  let question = $state("");
  let priority = $state<Priority>("routine");
  // One key per referral form; a new one after each success (OF-1).
  let idempotencyKey = newIdempotencyKey();
  // The one step being filled in, on one referral.
  let open = $state<{ id: string; next: NextEvent } | null>(null);
  let note = $state("");

  // A late answer for the patient the page has moved away from is dropped.
  let ticket = 0;
  async function load() {
    const mine = ++ticket;
    const forId = patientId;
    const res = await loadReferrals(forId);
    if (mine !== ticket || forId !== patientId) return;
    status = res.status;
    rows = res.referrals ?? [];
  }
  $effect(() => {
    void patientId;
    void refresh;
    status = null;
    rows = [];
    open = null;
    void load();
  });

  async function settle(res: { status: number; body: unknown }, ok: string) {
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = STRINGS[common];
    else if (res.status === 200 || res.status === 201) {
      message = ok;
      open = null;
      note = "";
      await load();
    } else if (res.status === 409) {
      message = STRINGS.referralChanged;
      open = null;
      await load();
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = STRINGS.error;
  }

  async function send(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(async () => {
      message = null;
      const res = await call(
        "POST",
        `/api/patients/${patientId}/referrals`,
        {
          referral: {
            recipient: recipient.trim(),
            reason: reason.trim(),
            question: question.trim(),
            priority,
          },
        },
        undefined,
        { idempotencyKey },
      );
      if (res.status === 201) {
        recipient = reason = question = "";
        priority = "routine";
        idempotencyKey = newIdempotencyKey();
      }
      await settle(res, STRINGS.referralSent);
    });
  }

  function start(r: StoredReferral, next: NextEvent) {
    message = null;
    note = "";
    // A step with no note is sent at once; one that needs a note opens its field first.
    if (next.needs_note) open = { id: r.id, next };
    else void step(r, next);
  }

  async function step(r: StoredReferral, next: NextEvent) {
    await busy.run(async () => {
      message = null;
      const res = await call(
        "POST",
        `/api/patients/${patientId}/referrals/${r.id}/events`,
        {
          event: next.event,
          expected_version: r.version,
          note: next.needs_note ? note.trim() : undefined,
        },
      );
      await settle(res, STRINGS.referralStepDone);
    });
  }
</script>

<section aria-labelledby="referrals-h">
  <h2 id="referrals-h">{STRINGS.referralsTitle}</h2>
  {#if status === null}
    <p>{STRINGS.loading}</p>
  {:else if status === 403}
    <p>{STRINGS.referralsNotShown}</p>
  {:else if status !== 200}
    <p>{STRINGS.referralsCouldNot}</p>
  {:else}
    {#if rows.length === 0}
      <p>{STRINGS.referralsNone}</p>
    {:else}
      <ul>
        {#each rows as r (r.id)}
          <li>
            <strong
              >{STRINGS.referralTo.replace(
                "{recipient}",
                r.referral.recipient,
              )}</strong
            >
            · {STRINGS.referralPriorities[r.referral.priority]}
            · {STRINGS.referralStates[r.state]}
            · {siteWhenText(r.recorded_at, siteZone)}
            {#if PROVISIONAL.includes(r.state)}
              <p role="note">{STRINGS.referralProvisional}</p>
            {/if}
            <p>{STRINGS.referralReason}: {r.referral.reason}</p>
            <p>{STRINGS.referralQuestion}: {r.referral.question}</p>
            {#each r.next as n (n.event)}
              <button
                disabled={busy.on}
                type="button"
                onclick={() => start(r, n)}
              >
                {STRINGS.referralEvents[
                  n.event as keyof typeof STRINGS.referralEvents
                ] ?? n.event}
                ({STRINGS.referralBy.replace(
                  "{by}",
                  STRINGS.referralRoles[n.by],
                )})
              </button>
            {/each}
            {#if open && open.id === r.id}
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  if (open) void step(r, open.next);
                }}
              >
                <label for="ref-note-{r.id}">{STRINGS.referralNote}</label>
                <input
                  id="ref-note-{r.id}"
                  bind:value={note}
                  required
                  maxlength="2000"
                  autocomplete="off"
                />
                <button disabled={busy.on} type="submit">
                  {STRINGS.referralEvents[
                    open.next.event as keyof typeof STRINGS.referralEvents
                  ] ?? open.next.event}
                </button>
              </form>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    <form onsubmit={send}>
      <label for="ref-recipient">{STRINGS.referralRecipient}</label>
      <input
        id="ref-recipient"
        bind:value={recipient}
        required
        maxlength="200"
        autocomplete="off"
      />
      <label for="ref-reason">{STRINGS.referralReason}</label>
      <input
        id="ref-reason"
        bind:value={reason}
        required
        maxlength="2000"
        autocomplete="off"
      />
      <label for="ref-question">{STRINGS.referralQuestion}</label>
      <input
        id="ref-question"
        bind:value={question}
        required
        maxlength="2000"
        autocomplete="off"
      />
      <label for="ref-priority">{STRINGS.referralPriority}</label>
      <select id="ref-priority" bind:value={priority}>
        {#each PRIORITIES as p (p)}
          <option value={p}>{STRINGS.referralPriorities[p]}</option>
        {/each}
      </select>
      <button disabled={busy.on} type="submit">{STRINGS.referralSend}</button>
    </form>
  {/if}
  {#if message}<p role="status">{message}</p>{/if}
</section>
