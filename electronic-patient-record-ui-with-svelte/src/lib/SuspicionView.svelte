<script lang="ts">
  // The suspicion on one referral (RS-1 to RS-7), read only. It says plainly when the pathway is a draft,
  // when the patient has not been told (so no one names the condition to them), whether a target day has
  // passed, and that the suspicion is open until an outcome is recorded. Text from the record is shown as
  // text. A referral with no suspicion offers the form to record one.
  import { siteWhenText } from "./access-log.js";
  import {
    loadSuspicion,
    outcomeText,
    targetMark,
    type SuspicionLoad,
  } from "./suspicions.js";
  import { TEXT } from "./i18n.js";
  import SuspicionRaise from "./SuspicionRaise.svelte";
  import SuspicionEnd from "./SuspicionEnd.svelte";

  let {
    patientId,
    referralId,
    siteZone = "UTC",
  }: { patientId: string; referralId: string; siteZone?: string } = $props();

  let res = $state<SuspicionLoad | null>(null);
  let reloads = $state(0);
  let ticket = 0;
  let shown = "";
  $effect(() => {
    void reloads;
    const mine = ++ticket;
    const [p, r] = [patientId, referralId];
    // Another patient's or referral's suspicion is never left on screen while this one loads.
    if (shown !== `${p}/${r}`) res = null;
    shown = `${p}/${r}`;
    void loadSuspicion(p, r).then((loaded) => {
      if (mine === ticket) res = loaded;
    });
  });
</script>

{#if res === null}
  <p>{TEXT.loading}</p>
{:else if res.status === 403}
  <p>{TEXT.suspicionNotShown}</p>
{:else if res.status !== 200 || !res.history}
  <p>{TEXT.suspicionCouldNot}</p>
{:else if res.history.current}
  {@const h = res.history}
  {@const c = h.current!}
  {@const ended = h.outcome !== null}
  <div>
    <p>
      <strong
        >{TEXT.suspicionTitle.replace(
          "{condition}",
          c.suspicion.condition,
        )}</strong
      >
    </p>
    {#if c.suspicion.patient_told === "not-told"}
      <p role="note"><strong>{TEXT.suspicionToldNo}</strong></p>
    {:else}
      <p>{TEXT.suspicionToldYes}</p>
    {/if}
    <p>
      {TEXT.suspicionPathway
        .replace("{title}", c.plan.title)
        .replace("{version}", String(c.plan.version))}
    </p>
    {#if !c.plan.approved}
      <p role="note">{TEXT.suspicionDraft}</p>
    {/if}
    {#if c.plan.latest_version > c.plan.version}
      <p>
        {TEXT.suspicionNewer.replace("{latest}", String(c.plan.latest_version))}
      </p>
    {/if}
    {#if c.plan.subtype}
      <p>
        {TEXT.suspicionSubtype
          .replace("{title}", c.plan.subtype.title)
          .replace("{name}", c.plan.subtype.name)
          .replace("{code}", c.plan.subtype.code)
          .replace("{scheme}", c.plan.subtype.scheme)}
      </p>
    {/if}
    <p>
      {TEXT.suspicionPriority.replace(
        "{priority}",
        TEXT.referralPriorities[c.plan.priority],
      )}
    </p>
    <p>
      {TEXT.suspicionPoint.replace(
        "{when}",
        siteWhenText(c.suspicion.point_of_suspicion, siteZone),
      )}
    </p>
    <p>
      {TEXT.suspicionClock.replace(
        "{from}",
        TEXT.suspicionClockFrom[c.plan.clock_from],
      )}
    </p>
    {#if c.plan.instruction}
      <p role="note">
        {TEXT.suspicionInstruction.replace("{instruction}", c.plan.instruction)}
      </p>
    {/if}
    {#if c.plan.targets.length > 0}
      <p>{TEXT.suspicionTargets.replace("{zone}", c.plan.time_zone)}</p>
      <ul>
        {#each c.plan.targets as t (t.id)}
          {@const mark = targetMark(t, ended)}
          <li>
            {TEXT.suspicionDue
              .replace("{text}", t.text)
              .replace("{day}", t.due_on)}
            {#if mark === "overdue"}
              · <strong>{TEXT.suspicionOverdue}</strong>
            {:else if mark === "passed-at-outcome"}
              · {TEXT.suspicionPassedAtOutcome}
            {/if}
          </li>
        {/each}
      </ul>
    {:else if c.plan.no_target_reason}
      <p>
        {TEXT.suspicionNoTarget.replace("{reason}", c.plan.no_target_reason)}
      </p>
    {/if}
    {#if h.outcome}
      <p>{outcomeText(h.outcome.outcome, TEXT.suspicionOutcomes)}</p>
    {:else}
      <p role="note">{TEXT.suspicionOpen}</p>
    {/if}
    {#if h.outcomes_in_error.length > 0}
      <p>
        {TEXT.suspicionInError.replace(
          "{n}",
          String(h.outcomes_in_error.length),
        )}
      </p>
    {/if}
    <SuspicionEnd
      {patientId}
      {referralId}
      version={c.version}
      outcomeSeq={h.outcome?.seq ?? null}
      done={() => reloads++}
    />
  </div>
{:else if res.pathways.length > 0}
  <SuspicionRaise
    {patientId}
    {referralId}
    pathways={res.pathways}
    done={() => reloads++}
  />
{/if}
