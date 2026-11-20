<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "#lib/api.js";
  import {
    overdue,
    reasonProblem,
    siteToday,
    sourceAnchor,
    TEAMS,
    type TaskItem,
    type TaskList,
  } from "#lib/tasks.js";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const busy = new Busy();
  let team = $state<string>("clinician");
  let items = $state<TaskItem[] | null>(null);
  let limit = $state(200);
  let more = $state(false);
  let zone = $state("UTC");
  let message = $state<string | null>(null);
  // The task whose close form is open, and what is in it.
  let closing = $state<string | null>(null);
  let outcome = $state<"done" | "cancelled">("done");
  let reason = $state("");
  let reasonError = $state<string | null>(null);
  const today = $derived(siteToday(new Date(), zone));

  async function load(keepMessage = false) {
    if (!keepMessage) message = null;
    const res = await call<TaskList>(
      "GET",
      `/api/tasks?team=${encodeURIComponent(team)}`,
    );
    if (res.status === 403) {
      // The list is only for those who hold the team's role: never read as "no tasks".
      items = null;
      message = TEXT.tkNotYours.replace(
        "{team}",
        TEXT.roleNames[team as keyof typeof TEXT.roleNames] ?? team,
      );
      return;
    }
    const problem = commonProblem(res.status);
    if (problem) {
      items = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      items = res.body.items;
      limit = res.body.limit;
      more = res.body.more;
      zone = res.body.site_time_zone;
    } else {
      items = null;
      message = TEXT.error;
    }
  }
  $effect(() => {
    void load();
  });

  function open(t: TaskItem) {
    closing = closing === t.id ? null : t.id;
    outcome = "done";
    reason = "";
    reasonError = null;
  }

  async function close(e: SubmitEvent, t: TaskItem) {
    e.preventDefault();
    const why = reasonProblem(reason);
    if (why) {
      reasonError =
        why === "required" ? TEXT.tkReasonRequired : TEXT.tkReasonTooLong;
      return;
    }
    reasonError = null;
    await busy.run(async () => {
      const res = await call("POST", `/api/tasks/${t.id}/close`, {
        expected_version: t.version,
        status: outcome,
        reason: reason.trim(),
      });
      const problem = commonProblem(res.status);
      if (problem) message = TEXT[problem];
      else if (res.status === 200) message = TEXT.tkClosed;
      // 409, or 422 for one closed meanwhile: someone else changed it. Read the list again to show what is true now.
      else if (res.status === 409 || res.status === 422)
        message = TEXT.tkChanged;
      else message = TEXT.error;
      if (!problem && [200, 409, 422].includes(res.status)) {
        closing = null;
        await load(true);
      }
    });
  }
</script>

<svelte:head><title>{TEXT.tkTitle}</title></svelte:head>
<h1>{TEXT.tkTitle}</h1>
<p>{TEXT.tkIntro}</p>
<form
  onsubmit={(e) => {
    e.preventDefault();
    void load();
  }}
>
  <label for="t-team">{TEXT.tkTeam}</label>
  <select id="t-team" bind:value={team}>
    {#each TEAMS as t (t)}
      <option value={t}>{TEXT.roleNames[t]}</option>
    {/each}
  </select>
  <button type="submit" disabled={busy.on}>{TEXT.tkShow}</button>
</form>
{#if message}<p role="status">{message}</p>{/if}
{#if items !== null}
  {#if items.length === 0}
    <p>{TEXT.tkNone}</p>
  {:else}
    <p>{TEXT.tkCount.replace("{n}", String(items.length))}</p>
    {#if more}
      <p>{TEXT.tkMore.replace("{n}", String(limit))}</p>
    {/if}
    <ul>
      {#each items as t (t.id)}
        <li>
          <strong>{t.title}</strong>
          <p>
            {overdue(t.due_on, today)
              ? TEXT.tkOverdue.replace("{day}", t.due_on)
              : TEXT.tkDue.replace("{day}", t.due_on)}
            · {TEXT.tkFrom[t.source.kind]}
            {#if t.reminded_on}
              · {TEXT.tkReminded.replace("{day}", t.reminded_on)}{/if}
          </p>
          <a
            href={localizePath(
              `/patients/${t.patient_id}${sourceAnchor(t.source.kind)}`,
              locale,
            )}
          >
            {TEXT.tkOpen}
          </a>
          <button
            type="button"
            disabled={busy.on}
            aria-expanded={closing === t.id}
            onclick={() => open(t)}
          >
            {TEXT.tkCloseLegend}
          </button>
          {#if closing === t.id}
            <form onsubmit={(e) => void close(e, t)}>
              <fieldset>
                <legend>{TEXT.tkCloseLegend}</legend>
                <label>
                  <input
                    type="radio"
                    name="t-outcome-{t.id}"
                    value="done"
                    bind:group={outcome}
                  />
                  {TEXT.tkDone}
                </label>
                <label>
                  <input
                    type="radio"
                    name="t-outcome-{t.id}"
                    value="cancelled"
                    bind:group={outcome}
                  />
                  {TEXT.tkCancel}
                </label>
              </fieldset>
              <label for="t-reason-{t.id}">{TEXT.tkReason}</label>
              <textarea
                id="t-reason-{t.id}"
                rows="2"
                bind:value={reason}
                aria-invalid={reasonError !== null}
                aria-describedby={reasonError
                  ? `t-reason-error-${t.id}`
                  : undefined}></textarea>
              {#if reasonError}<p id="t-reason-error-{t.id}">
                  {reasonError}
                </p>{/if}
              <button type="submit" disabled={busy.on}>{TEXT.tkSave}</button>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
