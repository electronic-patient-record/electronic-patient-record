<script lang="ts">
  // Whether a person can make one decision, now, as one person judged it (RF-8; UNREVIEWED wording). There is no score
  // and no verdict: each judgement is about one decision at one time, by someone named, with their reasons, and is shown
  // with its age because it goes out of date. A judgement is never copied forward or changed; a new view is a new
  // judgement, and one made by mistake is marked entered in error, with why (a clinician's act: the API refuses anyone
  // else). What a judgement does not record is shown beside it, and is not a verdict either. Where the latest views of two
  // people differ about one decision, the page says so and takes no side. Shown only when the API lets the reader list them.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import { newIdempotencyKey } from "./idempotency.js";
  import { TEXT } from "./i18n.js";
  import { gapText } from "./decision-ability.js";

  type Known = "yes" | "no" | "not-known";
  type Communicates = "speaks" | "other-means" | "no" | "not-known";
  type Decides = "can" | "cannot" | "not-known";
  type Help =
    | "interpreter"
    | "communication-aid"
    | "time"
    | "place"
    | "supporter"
    | "information"
    | "other";
  interface Judgement {
    about: string;
    at: string;
    conscious: Known;
    communicates: Communicates;
    decides: Decides;
    help_given: Help[];
    help_words?: string;
    because: string;
    by: string;
  }
  interface Listed {
    id: string;
    version: number;
    state: "recorded" | "entered-in-error";
    judgement: Judgement;
    age_minutes: number | null;
    gaps: string[];
    from_merged_record: boolean;
  }

  const busy = new Busy();
  let { patientId }: { patientId: string } = $props();

  const KNOWN: Known[] = ["yes", "no", "not-known"];
  const COMMUNICATES: Communicates[] = [
    "speaks",
    "other-means",
    "no",
    "not-known",
  ];
  const DECIDES: Decides[] = ["can", "cannot", "not-known"];
  const HELP: Help[] = [
    "interpreter",
    "communication-aid",
    "time",
    "place",
    "supporter",
    "information",
    "other",
  ];

  let loaded = $state<{ judgements: Listed[]; disputed: string[] } | null>(
    null,
  );
  let loadProblem = $state<string | null>(null);
  let message = $state<string | null>(null);

  let about = $state("");
  let at = $state("");
  let conscious = $state<Known | "">("");
  let communicates = $state<Communicates | "">("");
  let decides = $state<Decides | "">("");
  let help = $state<Help[]>([]);
  let helpWords = $state("");
  let because = $state("");
  let by = $state("");
  let idempotencyKey = newIdempotencyKey();

  let marking = $state<string | null>(null);
  let why = $state("");

  async function load() {
    const res = await call<{ judgements: Listed[]; disputed: string[] }>(
      "GET",
      `/api/patients/${patientId}/decision-ability`,
    );
    if (res.status === 200 && res.body) {
      loaded = res.body;
      loadProblem = null;
    } else if (res.status === 403) {
      loaded = null;
      loadProblem = null;
    } else {
      loaded = null;
      loadProblem = TEXT.daUnreadable;
    }
  }

  $effect(() => {
    void patientId;
    void load();
  });

  /** How old a judgement is, in words; it goes out of date by its age. */
  function ageText(minutes: number | null): string {
    if (minutes === null) return TEXT.daAgeUnknown;
    if (minutes < 60) return TEXT.daAgeMinutes.replace("{n}", String(minutes));
    if (minutes < 48 * 60)
      return TEXT.daAgeHours.replace("{n}", String(Math.floor(minutes / 60)));
    return TEXT.daAgeDays.replace(
      "{n}",
      String(Math.floor(minutes / (24 * 60))),
    );
  }

  function toggle(h: Help, on: boolean) {
    help = on
      ? [...help.filter((x) => x !== h), h]
      : help.filter((x) => x !== h);
  }

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => recordNow());
  }

  async function recordNow() {
    message = null;
    const when = at ? new Date(at) : null;
    const unanswered = [
      [about.trim(), TEXT.daAbout],
      [when && !Number.isNaN(when.getTime()) ? "set" : "", TEXT.daAt],
      [conscious, TEXT.daConscious],
      [communicates, TEXT.daCommunicates],
      [decides, TEXT.daDecides],
      [help.includes("other") ? helpWords.trim() : "set", TEXT.daHelpWords],
      [because.trim(), TEXT.daBecause],
      [by.trim(), TEXT.daBy],
    ]
      .filter(([v]) => v === "")
      .map(([, label]) => label);
    if (unanswered.length > 0) {
      message = TEXT.rfUnanswered.replace("{fields}", unanswered.join(", "));
      return;
    }
    const judgement = {
      about: about.trim(),
      at: when!.toISOString(),
      conscious,
      communicates,
      decides,
      help_given: HELP.filter((h) => help.includes(h)),
      ...(helpWords.trim() ? { help_words: helpWords.trim() } : {}),
      because: because.trim(),
      by: by.trim(),
    };
    const res = await call(
      "POST",
      `/api/patients/${patientId}/decision-ability`,
      { judgement },
      undefined,
      { idempotencyKey },
    );
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = TEXT[common];
    else if (res.status === 201) {
      about = at = helpWords = because = by = "";
      conscious = communicates = decides = "";
      help = [];
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
        `/api/patients/${patientId}/decision-ability/${r.id}/entered-in-error`,
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
</script>

{#snippet choose<T extends string>(
  id: string,
  label: string,
  value: T | "",
  options: T[],
  words: Record<string, string>,
  set: (v: T) => void,
)}
  <label for={id}>{label}</label>
  <select
    {id}
    {value}
    onchange={(e) => set((e.currentTarget as HTMLSelectElement).value as T)}
  >
    <option value="" disabled>{TEXT.anChoose}</option>
    {#each options as o (o)}
      <option value={o}>{words[o]}</option>
    {/each}
  </select>
{/snippet}

{#if loaded || loadProblem}
  <section aria-labelledby="decision-ability-h">
    <h2 id="decision-ability-h">{TEXT.daTitle}</h2>
    <p role="note">{TEXT.daIntro}</p>
    {#if loadProblem}
      <!-- Louder than "none recorded", but not an alert: the page keeps those for what the reader just did. -->
      <p><strong>{loadProblem}</strong></p>
    {:else if loaded}
      {#each loaded.disputed as d (d)}
        <p class="disputed">
          <strong>{TEXT.daDisputed.replace("{about}", d)}</strong>
        </p>
      {/each}
      {#if loaded.judgements.length === 0}
        <p>{TEXT.anNone}</p>
      {:else}
        <ul>
          {#each loaded.judgements as r (r.id)}
            <li>
              <strong>{r.judgement.about}</strong>
              · {(TEXT.daDecidesWords as Record<string, string>)[
                r.judgement.decides
              ]}
              · {ageText(r.age_minutes)} · {r.judgement.by}
              · {TEXT.daBecause}: {r.judgement.because}
              {#if r.from_merged_record}· {TEXT.refusalFromMerged}{/if}
              {#if r.state === "entered-in-error"}
                · {TEXT.anEnteredInError}
              {:else}
                {#if r.gaps.length > 0}
                  <ul>
                    {#each r.gaps as g (g)}<li>{gapText(g, TEXT)}</li>{/each}
                  </ul>
                {/if}
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
                    <label for="da-why">{TEXT.rfErrorWhy}</label>
                    <input id="da-why" bind:value={why} autocomplete="off" />
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
      <h3>{TEXT.daRecord}</h3>
      <form onsubmit={record}>
        <label for="da-about">{TEXT.daAbout}</label>
        <input id="da-about" bind:value={about} autocomplete="off" />
        <label for="da-at">{TEXT.daAt}</label>
        <input id="da-at" type="datetime-local" bind:value={at} />
        {@render choose(
          "da-conscious",
          TEXT.daConscious,
          conscious,
          KNOWN,
          TEXT.anKnown,
          (v) => (conscious = v),
        )}
        {@render choose(
          "da-communicates",
          TEXT.daCommunicates,
          communicates,
          COMMUNICATES,
          TEXT.daCommunicatesWords,
          (v) => (communicates = v),
        )}
        <fieldset>
          <legend>{TEXT.daHelp}</legend>
          {#each HELP as h (h)}
            <label>
              <input
                type="checkbox"
                checked={help.includes(h)}
                onchange={(e) =>
                  toggle(h, (e.currentTarget as HTMLInputElement).checked)}
              />
              {(TEXT.daHelpWordsOf as Record<string, string>)[h]}
            </label>
          {/each}
        </fieldset>
        <label for="da-help-words">{TEXT.daHelpWords}</label>
        <input id="da-help-words" bind:value={helpWords} autocomplete="off" />
        {@render choose(
          "da-decides",
          TEXT.daDecides,
          decides,
          DECIDES,
          TEXT.daDecidesWords,
          (v) => (decides = v),
        )}
        <label for="da-because">{TEXT.daBecause}</label>
        <input id="da-because" bind:value={because} autocomplete="off" />
        <label for="da-by">{TEXT.daBy}</label>
        <input id="da-by" bind:value={by} autocomplete="off" />
        <button type="submit" disabled={busy.on}>{TEXT.daSave}</button>
      </form>
    {/if}
    {#if message}<p role="status">{message}</p>{/if}
  </section>
{/if}
