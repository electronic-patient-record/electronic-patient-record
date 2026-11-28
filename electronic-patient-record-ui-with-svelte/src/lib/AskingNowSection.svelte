<script lang="ts">
  // What a person was asked now, and leaving against advice (RF-10, with EC-6; UNREVIEWED wording). What was offered and
  // explained, what they refused and accepted, whether they were told they may change their mind, and where. Every
  // yes-or-no allows "not known", and one button marks each unanswered question not known, so nothing is guessed. A
  // record is never changed; one made by mistake is marked entered in error, with why (a clinician's act: the API refuses
  // anyone else, and the screen shows its answer). Nothing here says whether a refusal stands. Shown only when the API
  // lets the reader list these records.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import { newIdempotencyKey } from "./idempotency.js";
  import { TEXT } from "./i18n.js";

  type Known = "yes" | "no" | "not-known";
  interface Asked {
    offered: string;
    explained: {
      risks: Known;
      benefits: Known;
      alternatives: Known;
      language?: string;
      helped_by?: string;
    };
    refused?: string;
    accepted?: string;
    told_may_change_mind: Known;
    in_private: Known;
    with_whom?: string;
    leaving_against_advice: boolean;
    written: Known;
    signed: Known;
    witnessed: Known;
    at: string;
    by: string;
  }
  interface Listed {
    id: string;
    version: number;
    state: "recorded" | "entered-in-error";
    reason?: string | null;
    asked: Asked;
    from_merged_record: boolean;
    /** What the place's pack asks for that this record does not show as done; null with no pack in force. */
    pack_gaps?: string[] | null;
  }
  interface InForce {
    id: string;
    place: string;
    law_as_of: string;
    lawyer?: string;
    clinician?: string;
    reviewed_on?: string;
    review: "unreviewed" | "lapsed" | "current";
  }

  const busy = new Busy();
  let { patientId }: { patientId: string } = $props();

  const KNOWN: Known[] = ["yes", "no", "not-known"];
  let loaded = $state<{
    records: Listed[];
    left_against_advice: number;
    pack?: InForce | null;
  } | null>(null);
  let loadProblem = $state<string | null>(null);
  let message = $state<string | null>(null);

  let offered = $state("");
  let risks = $state<Known | "">("");
  let benefits = $state<Known | "">("");
  let alternatives = $state<Known | "">("");
  let language = $state("");
  let helpedBy = $state("");
  let refused = $state("");
  let accepted = $state("");
  let mayChange = $state<Known | "">("");
  let inPrivate = $state<Known | "">("");
  let withWhom = $state("");
  let leaving = $state(false);
  let written = $state<Known | "">("");
  let signed = $state<Known | "">("");
  let witnessed = $state<Known | "">("");
  let at = $state("");
  let by = $state("");
  let idempotencyKey = newIdempotencyKey();

  let marking = $state<string | null>(null);
  let why = $state("");

  async function load() {
    const res = await call<{
      records: Listed[];
      left_against_advice: number;
      pack?: InForce | null;
    }>("GET", `/api/patients/${patientId}/asking-now`);
    if (res.status === 200 && res.body) {
      loaded = res.body;
      loadProblem = null;
    } else if (res.status === 403) {
      loaded = null;
      loadProblem = null;
    } else {
      loaded = null;
      loadProblem = TEXT.anUnreadable;
    }
  }

  $effect(() => {
    void patientId;
    void load();
  });

  function notKnownForTheRest() {
    for (const [get, set] of [
      [() => risks, (v: Known) => (risks = v)],
      [() => benefits, (v: Known) => (benefits = v)],
      [() => alternatives, (v: Known) => (alternatives = v)],
      [() => mayChange, (v: Known) => (mayChange = v)],
      [() => inPrivate, (v: Known) => (inPrivate = v)],
      [() => written, (v: Known) => (written = v)],
      [() => signed, (v: Known) => (signed = v)],
      [() => witnessed, (v: Known) => (witnessed = v)],
    ] as [() => Known | "", (v: Known) => void][]) {
      if (get() === "") set("not-known");
    }
  }

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => recordNow());
  }

  async function recordNow() {
    message = null;
    const when = at ? new Date(at) : null;
    const unanswered = [
      [offered.trim(), TEXT.anOffered],
      [risks, TEXT.anRisks],
      [benefits, TEXT.anBenefits],
      [alternatives, TEXT.anAlternatives],
      [mayChange, TEXT.anMayChange],
      [inPrivate, TEXT.anInPrivate],
      [written, TEXT.anWritten],
      [signed, TEXT.anSigned],
      [witnessed, TEXT.anWitnessed],
      [when && !Number.isNaN(when.getTime()) ? "set" : "", TEXT.anAt],
      [by.trim(), TEXT.anBy],
    ]
      .filter(([v]) => v === "")
      .map(([, label]) => label);
    if (unanswered.length > 0) {
      message = TEXT.rfUnanswered.replace("{fields}", unanswered.join(", "));
      return;
    }
    const asked = {
      offered: offered.trim(),
      explained: {
        risks,
        benefits,
        alternatives,
        ...(language.trim() ? { language: language.trim() } : {}),
        ...(helpedBy.trim() ? { helped_by: helpedBy.trim() } : {}),
      },
      ...(refused.trim() ? { refused: refused.trim() } : {}),
      ...(accepted.trim() ? { accepted: accepted.trim() } : {}),
      told_may_change_mind: mayChange,
      in_private: inPrivate,
      ...(inPrivate !== "yes" && withWhom.trim()
        ? { with_whom: withWhom.trim() }
        : {}),
      leaving_against_advice: leaving,
      written,
      signed,
      witnessed,
      at: when!.toISOString(),
      by: by.trim(),
    };
    const res = await call(
      "POST",
      `/api/patients/${patientId}/asking-now`,
      { asked },
      undefined,
      { idempotencyKey },
    );
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = TEXT[common];
    else if (res.status === 201) {
      offered =
        language =
        helpedBy =
        refused =
        accepted =
        withWhom =
        at =
        by =
          "";
      risks =
        benefits =
        alternatives =
        mayChange =
        inPrivate =
        written =
        signed =
        witnessed =
          "";
      leaving = false;
      idempotencyKey = newIdempotencyKey();
      message = TEXT.rfSaved;
      await load();
    } else if (res.status === 422 && body?.errors)
      message = body.errors.join(" ");
    else message = TEXT.error;
  }

  async function markError(event: SubmitEvent, r: Listed) {
    event.preventDefault();
    await busy.run(async () => {
      message = null;
      if (!why.trim()) {
        message = TEXT.rfWhyNeeded;
        return;
      }
      const res = await call(
        "POST",
        `/api/patients/${patientId}/asking-now/${r.id}/entered-in-error`,
        { expected_version: r.version, reason: why.trim() },
      );
      const common = commonProblem(res.status);
      const body = res.body as { errors?: string[] } | null;
      if (common) message = TEXT[common];
      else if (res.status === 200) {
        marking = null;
        why = "";
        message = TEXT.rfSaved;
        await load();
      } else if (res.status === 409) {
        marking = null;
        message = TEXT.rfChanged;
        await load();
      } else if (res.status === 422 && body?.errors)
        message = body.errors.join(" ");
      else message = TEXT.error;
    });
  }

  function whenText(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime())
      ? iso
      : d.toISOString().slice(0, 16).replace("T", " ");
  }
</script>

{#snippet known(
  id: string,
  label: string,
  value: Known | "",
  set: (v: Known) => void,
)}
  <label for={id}>{label}</label>
  <select
    {id}
    {value}
    onchange={(e) => set((e.currentTarget as HTMLSelectElement).value as Known)}
  >
    <option value="" disabled>{TEXT.anChoose}</option>
    {#each KNOWN as k (k)}
      <option value={k}>{(TEXT.anKnown as Record<string, string>)[k]}</option>
    {/each}
  </select>
{/snippet}

{#if loaded || loadProblem}
  <section aria-labelledby="asking-now-h">
    <h2 id="asking-now-h">{TEXT.anTitle}</h2>
    <p role="note">{TEXT.anIntro}</p>
    {#if loadProblem}
      <!-- Louder than "none recorded", but not an alert: the page keeps those for what the reader just did. -->
      <p><strong>{loadProblem}</strong></p>
    {:else if loaded}
      {#if loaded.pack}
        <p>
          {TEXT.anPack
            .replace("{place}", loaded.pack.place)
            .replace("{day}", loaded.pack.law_as_of)}
          {(TEXT.anPackReview as Record<string, string>)[loaded.pack.review]}
        </p>
      {:else}
        <p>{TEXT.anNoPack}</p>
      {/if}
      {#if loaded.left_against_advice > 0}
        <p>
          {TEXT.anLeftCount.replace("{n}", String(loaded.left_against_advice))}
        </p>
      {/if}
      {#if loaded.records.length === 0}
        <p>{TEXT.anNone}</p>
      {:else}
        <ul>
          {#each loaded.records as r (r.id)}
            <li>
              <strong>{r.asked.offered}</strong>
              · {whenText(r.asked.at)} · {r.asked.by}
              {#if r.asked.refused}· {TEXT.anRefused}: {r.asked.refused}{/if}
              {#if r.asked.accepted}· {TEXT.anAccepted}: {r.asked.accepted}{/if}
              {#if r.asked.leaving_against_advice}· {TEXT.anLeaving}{/if}
              {#if r.from_merged_record}· {TEXT.refusalFromMerged}{/if}
              {#if r.state !== "entered-in-error" && r.pack_gaps && r.pack_gaps.length > 0}
                <p>
                  {TEXT.anPackGaps}
                  {r.pack_gaps
                    .map(
                      (g) =>
                        (TEXT.anPackWords as Record<string, string>)[g] ?? g,
                    )
                    .join(", ")}
                </p>
              {/if}
              {#if r.state === "entered-in-error"}
                · {TEXT.anEnteredInError}
              {:else}
                <button
                  type="button"
                  onclick={() => {
                    marking = r.id;
                    why = "";
                    message = null;
                  }}
                >
                  {TEXT.rfError}
                </button>
                {#if marking === r.id}
                  <form onsubmit={(e) => markError(e, r)}>
                    <label for="an-why">{TEXT.rfErrorWhy}</label>
                    <input id="an-why" bind:value={why} autocomplete="off" />
                    <button type="submit" disabled={busy.on}>
                      {TEXT.rfSaveChange}
                    </button>
                  </form>
                {/if}
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <h3>{TEXT.anRecord}</h3>
      <form onsubmit={record}>
        <label for="an-offered">{TEXT.anOffered}</label>
        <input id="an-offered" bind:value={offered} autocomplete="off" />
        {@render known("an-risks", TEXT.anRisks, risks, (v) => (risks = v))}
        {@render known(
          "an-benefits",
          TEXT.anBenefits,
          benefits,
          (v) => (benefits = v),
        )}
        {@render known(
          "an-alternatives",
          TEXT.anAlternatives,
          alternatives,
          (v) => (alternatives = v),
        )}
        <label for="an-language">{TEXT.anLanguage}</label>
        <input id="an-language" bind:value={language} autocomplete="off" />
        <label for="an-helped">{TEXT.anHelpedBy}</label>
        <input id="an-helped" bind:value={helpedBy} autocomplete="off" />
        <label for="an-refused">{TEXT.anRefused}</label>
        <input id="an-refused" bind:value={refused} autocomplete="off" />
        <label for="an-accepted">{TEXT.anAccepted}</label>
        <input id="an-accepted" bind:value={accepted} autocomplete="off" />
        {@render known(
          "an-may-change",
          TEXT.anMayChange,
          mayChange,
          (v) => (mayChange = v),
        )}
        {@render known(
          "an-private",
          TEXT.anInPrivate,
          inPrivate,
          (v) => (inPrivate = v),
        )}
        {#if inPrivate !== "yes"}
          <label for="an-with">{TEXT.anWithWhom}</label>
          <input id="an-with" bind:value={withWhom} autocomplete="off" />
        {/if}
        <label>
          <input type="checkbox" bind:checked={leaving} />
          {TEXT.anLeaving}
        </label>
        {@render known(
          "an-written",
          TEXT.anWritten,
          written,
          (v) => (written = v),
        )}
        {@render known("an-signed", TEXT.anSigned, signed, (v) => (signed = v))}
        {@render known(
          "an-witnessed",
          TEXT.anWitnessed,
          witnessed,
          (v) => (witnessed = v),
        )}
        <label for="an-at">{TEXT.anAt}</label>
        <input id="an-at" type="datetime-local" bind:value={at} />
        <label for="an-by">{TEXT.anBy}</label>
        <input id="an-by" bind:value={by} autocomplete="off" />
        <button type="button" onclick={notKnownForTheRest}>
          {TEXT.rfNotKnownRest}
        </button>
        <button type="submit" disabled={busy.on}>{TEXT.anSave}</button>
      </form>
    {/if}
    {#if message}<p role="status">{message}</p>{/if}
  </section>
{/if}
