<script lang="ts">
  // The emergency sheet (RF-7): every refusal of treatment on the record, in large type, one key from the record, and
  // printable. What is refused and what is accepted, when it applies, who said it and when, its form and where the paper
  // is, any doubt and any withdrawal heard. "None recorded" is never "none", and a list that could not be read is louder
  // than an empty one. The wording is the author's and UNREVIEWED, and the page says so.
  import { untrack } from "svelte";
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import {
    NATIONAL_NUMBER_SYSTEM,
    nationalNumberDisplay,
    type Stored,
  } from "#lib/patient.js";
  import {
    loadRefusals,
    type ListedRefusal,
    type RefusalLoad,
  } from "#lib/treatment-refusals.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const id = $derived(page.params.id);

  let stored = $state<Stored | null>(null);
  let refusals = $state<RefusalLoad | null>(null);
  let message = $state<string | null>(null);
  // The page is reused when the address moves to another patient (CQ-6, H-016).
  let ticket = 0;

  async function load() {
    const forId = id ?? "";
    const mine = ++ticket;
    stored = null;
    refusals = null;
    message = null;
    const [p, r] = await Promise.all([
      call<Stored>("GET", `/api/patients/${forId}`),
      loadRefusals(forId),
    ]);
    if (mine !== ticket) return;
    const common = commonProblem(p.status);
    if (common) message = TEXT[common];
    else if (p.status === 404) message = TEXT.notFound;
    else if (p.status !== 200) message = TEXT.error;
    else stored = p.body;
    refusals = r;
  }

  $effect(() => {
    void id;
    untrack(() => void load());
  });

  // Those that stand first, then those in doubt, then those withdrawn; none entered in error.
  const rank = { stands: 0, doubted: 1, withdrawn: 2 } as const;
  const shown = $derived(
    refusals?.status === 200 && refusals.data
      ? refusals.data.refusals
          .filter((r) => r.refusal.state !== "entered-in-error")
          .sort(
            (a, b) =>
              rank[a.refusal.state as keyof typeof rank] -
              rank[b.refusal.state as keyof typeof rank],
          )
      : [],
  );

  const name = $derived.by(() => {
    const n = stored?.patient.names[0];
    return [n?.family?.toUpperCase(), (n?.given ?? []).join(" ")]
      .filter(Boolean)
      .join(", ");
  });
  const national = $derived(
    stored?.patient.identifiers.find((i) => i.system === NATIONAL_NUMBER_SYSTEM)
      ?.value,
  );
  const known = (k: string | undefined) =>
    k ? ((TEXT.emKnown as Record<string, string>)[k] ?? k) : TEXT.emNotSaid;
  const form = (r: ListedRefusal["refusal"]) => {
    const f = r.form;
    if (!f) return TEXT.emNotSaid;
    return [
      `${TEXT.emWritten}: ${known(f.written)}`,
      `${TEXT.emSigned}: ${known(f.signed)}`,
      `${TEXT.emWitnessed}: ${known(f.witnessed)}${f.witnesses ? ` (${f.witnesses})` : ""}`,
      `${TEXT.emSeen}: ${(TEXT.emSeenWords as Record<string, string>)[f.seen] ?? f.seen}`,
    ].join(" · ");
  };
</script>

<svelte:head><title>{TEXT.emTitle}</title></svelte:head>

<div class="emergency">
  <h1>{TEXT.emTitle}</h1>
  <p role="note">{TEXT.emUnreviewed}</p>
  <p class="no-print">
    <a href={localizePath(`/patients/${id}`, locale)}>{TEXT.summaryBack}</a>
    ·
    <button type="button" onclick={() => window.print()}>{TEXT.emPrint}</button>
  </p>

  {#if message}
    <p role="alert">{message}</p>
  {/if}

  {#if stored}
    <p class="who">
      <strong>{name || TEXT.notAvailable}</strong>
      · {TEXT.bannerBorn}: {stored.patient.birth_date ?? TEXT.bannerNoBirthDate}
      · {TEXT.bannerNationalNumber}: {national
        ? nationalNumberDisplay(national)
        : TEXT.bannerNone}
    </p>
    {#if stored.patient.temporary_identity}
      <p class="loud" role="alert">{TEXT.bannerTemporaryIdentity}</p>
    {/if}
  {/if}

  <section aria-labelledby="em-refusals-h">
    <h2 id="em-refusals-h">{TEXT.emRefusals}</h2>
    {#if refusals === null}
      <p role="status" aria-busy="true">{TEXT.loading}</p>
    {:else if refusals.status === 403}
      <p>{TEXT.emNotShown}</p>
    {:else if refusals.status !== 200 || !refusals.data}
      <p class="loud" role="alert">{TEXT.emLoadFailed}</p>
    {:else if refusals.data.summary.state === "none-recorded"}
      <p>{TEXT.emNoneRecorded}</p>
    {:else}
      {#each shown as r (r.id)}
        {@const x = r.refusal}
        <article class:loud={x.state !== "withdrawn"}>
          <h3>{x.treatment}</h3>
          <p>
            <strong>
              {(TEXT.emStates as Record<string, string>)[x.state]}
            </strong>
            {#if x.kind}· {(TEXT.emKinds as Record<string, string>)[
                x.kind
              ]}{/if}
            {#if x.identity === "uncertain"}· {TEXT.refusalIdentityUncertain}{/if}
            {#if r.from_merged_record}· {TEXT.refusalFromMerged}{/if}
          </p>
          <dl>
            <dt>{TEXT.emAccepts}</dt>
            <dd>{x.accepts ?? TEXT.emNotSaid}</dd>
            <dt>{TEXT.emCircumstances}</dt>
            <dd>{x.circumstances ?? TEXT.emNotSaid}</dd>
            <dt>{TEXT.emLifeAtRisk}</dt>
            <dd>
              {x.if_life_at_risk
                ? (TEXT.emLifeWords as Record<string, string>)[
                    x.if_life_at_risk
                  ]
                : TEXT.emNotSaid}
            </dd>
            <dt>{TEXT.emSaidBy}</dt>
            <dd>
              {x.said_by
                ? (TEXT.emSaidByWords as Record<string, string>)[x.said_by]
                : TEXT.emNotSaid}
              {#if x.power_seen}· {TEXT.emPowerSeen}: {known(x.power_seen)}{/if}
            </dd>
            <dt>{TEXT.emMadeOn}</dt>
            <dd>{x.made_on ?? TEXT.emNotKnown}</dd>
            <dt>{TEXT.emConfirmedOn}</dt>
            <dd>{x.confirmed_on ?? TEXT.emNotKnown}</dd>
            <dt>{TEXT.emForm}</dt>
            <dd>{form(x)}</dd>
            <dt>{TEXT.emKept}</dt>
            <dd>{x.form?.kept ?? TEXT.emNotSaid}</dd>
            {#if x.doubt}
              <dt>{TEXT.emDoubt}</dt>
              <dd>{x.doubt}</dd>
            {/if}
            {#if x.withdrawal}
              <dt>{TEXT.emWithdrawal}</dt>
              <dd>
                {(TEXT.emHow as Record<string, string>)[x.withdrawal.how]}
                · {x.withdrawal.noticed_by} · {x.withdrawal.at}
              </dd>
            {/if}
          </dl>
        </article>
      {/each}
    {/if}
  </section>
</div>
