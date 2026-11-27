<script lang="ts">
  // Who speaks for a person who cannot, for a decision (RF-9; UNREVIEWED wording): an attorney, a guardian or deputy, a
  // court, each parent apart, a relative with no power, or nobody. Nothing here decides who wins. A power is shown as one
  // only when someone saw its paper; one only said to exist is shown as claimed and not a power. A relative's word is a
  // source. Two people who disagree are two entries, and where powers that were seen disagree the page says so. A change
  // is a new version; one entered in error stays and counts for nothing (a clinician's act: the API refuses anyone else).
  // Shown only when the API lets the reader list them.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import { newIdempotencyKey } from "./idempotency.js";
  import { TEXT } from "./i18n.js";

  type Role =
    "attorney" | "guardian" | "court" | "parent" | "relative" | "nobody";
  type Paper = "original" | "copy" | "register" | "not-seen";
  type Says = "agrees" | "refuses" | "undecided";
  interface View {
    about: string;
    says: Says;
  }
  interface Speaker {
    role: Role;
    name?: string;
    paper?: Paper;
    seen_by?: string;
    covers?: string;
    views: View[];
  }
  interface Listed {
    id: string;
    version: number;
    state: "stands" | "entered-in-error";
    speaker: Speaker;
    from_merged_record: boolean;
  }
  interface Summary {
    powers_seen: string[];
    claimed_not_seen: string[];
    without_power: string[];
    nobody_recorded: boolean;
    contradicts: boolean;
    powers_disagree: string[];
  }

  const busy = new Busy();
  let { patientId }: { patientId: string } = $props();

  const ROLES: Role[] = [
    "attorney",
    "guardian",
    "court",
    "parent",
    "relative",
    "nobody",
  ];
  const PAPERS: Paper[] = ["original", "copy", "register", "not-seen"];
  const SAYS: Says[] = ["agrees", "refuses", "undecided"];
  const canHoldPower = (r: Role | "") =>
    r === "attorney" || r === "guardian" || r === "court" || r === "parent";

  let loaded = $state<{ speakers: Listed[]; summary: Summary } | null>(null);
  let loadProblem = $state<string | null>(null);
  let message = $state<string | null>(null);

  let role = $state<Role | "">("");
  let name = $state("");
  let paper = $state<Paper | "">("");
  let seenBy = $state("");
  let covers = $state("");
  let views = $state<{ about: string; says: Says | "" }[]>([]);
  let idempotencyKey = newIdempotencyKey();
  const seen = $derived(
    paper === "original" || paper === "copy" || paper === "register",
  );

  let marking = $state<string | null>(null);
  let why = $state("");

  async function load() {
    const res = await call<{ speakers: Listed[]; summary: Summary }>(
      "GET",
      `/api/patients/${patientId}/speakers`,
    );
    if (res.status === 200 && res.body) {
      loaded = res.body;
      loadProblem = null;
    } else if (res.status === 403) {
      loaded = null;
      loadProblem = null;
    } else {
      loaded = null;
      loadProblem = TEXT.wsUnreadable;
    }
  }

  $effect(() => {
    void patientId;
    void load();
  });

  /** What an entry's power is, in words: seen, claimed and not a power, or none. */
  function powerText(s: Speaker): string {
    if (s.role === "relative") return TEXT.wsNoPower;
    if (s.role === "nobody") return "";
    if (s.paper === "not-seen" || !s.paper) return TEXT.wsClaimed;
    return TEXT.wsSeen
      .replace(
        "{paper}",
        (TEXT.wsPaperWords as Record<string, string>)[s.paper] ?? s.paper,
      )
      .replace("{by}", s.seen_by ?? "");
  }

  async function record(event: SubmitEvent) {
    event.preventDefault();
    await busy.run(() => recordNow());
  }

  async function recordNow() {
    message = null;
    const power = canHoldPower(role);
    const unanswered = [
      [role, TEXT.wsRole],
      [role === "nobody" ? "set" : name.trim(), TEXT.wsName],
      [power ? paper : "set", TEXT.wsPaper],
      [power && seen ? seenBy.trim() : "set", TEXT.wsSeenBy],
      ...views.flatMap((v) => [
        [v.about.trim(), TEXT.wsAbout],
        [v.says, TEXT.wsSays],
      ]),
    ]
      .filter(([v]) => v === "")
      .map(([, label]) => label);
    if (unanswered.length > 0) {
      message = TEXT.rfUnanswered.replace(
        "{fields}",
        [...new Set(unanswered)].join(", "),
      );
      return;
    }
    const speaker = {
      role,
      ...(role !== "nobody" ? { name: name.trim() } : {}),
      ...(power ? { paper } : {}),
      ...(power && seen ? { seen_by: seenBy.trim() } : {}),
      ...(power && seen && covers.trim() ? { covers: covers.trim() } : {}),
      views:
        role === "nobody"
          ? []
          : views.map((v) => ({ about: v.about.trim(), says: v.says })),
    };
    const res = await call(
      "POST",
      `/api/patients/${patientId}/speakers`,
      { speaker },
      undefined,
      { idempotencyKey },
    );
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) message = TEXT[common];
    else if (res.status === 201) {
      role = paper = "";
      name = seenBy = covers = "";
      views = [];
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
        "PUT",
        `/api/patients/${patientId}/speakers/${r.id}`,
        {
          speaker: r.speaker,
          expected_version: r.version,
          state: "entered-in-error",
          reason: why.trim(),
        },
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

{#if loaded || loadProblem}
  <section aria-labelledby="speakers-h">
    <h2 id="speakers-h">{TEXT.wsTitle}</h2>
    <p role="note">{TEXT.wsIntro}</p>
    {#if loadProblem}
      <!-- Louder than "none recorded", but not an alert: the page keeps those for what the reader just did. -->
      <p><strong>{loadProblem}</strong></p>
    {:else if loaded}
      {#if loaded.summary.contradicts}
        <p><strong>{TEXT.wsContradicts}</strong></p>
      {/if}
      {#each loaded.summary.powers_disagree as d (d)}
        <p><strong>{TEXT.wsDisagree.replace("{about}", d)}</strong></p>
      {/each}
      {#if loaded.speakers.length === 0}
        <p>{TEXT.anNone}</p>
      {:else}
        <ul>
          {#each loaded.speakers as r (r.id)}
            <li>
              <strong>
                {(TEXT.wsRoleWords as Record<string, string>)[r.speaker.role]}
              </strong>
              {#if r.speaker.name}· {r.speaker.name}{/if}
              {#if powerText(r.speaker)}· {powerText(r.speaker)}{/if}
              {#if r.speaker.covers}· {TEXT.wsCovers}: {r.speaker.covers}{/if}
              {#if r.from_merged_record}· {TEXT.refusalFromMerged}{/if}
              {#if r.speaker.views.length > 0}
                <ul>
                  {#each r.speaker.views as v (v.about)}
                    <li>
                      {v.about}: {(TEXT.wsSaysWords as Record<string, string>)[
                        v.says
                      ]}
                    </li>
                  {/each}
                </ul>
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
                    <label for="ws-why">{TEXT.rfErrorWhy}</label>
                    <input id="ws-why" bind:value={why} autocomplete="off" />
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
      <h3>{TEXT.wsRecord}</h3>
      <form onsubmit={record}>
        <label for="ws-role">{TEXT.wsRole}</label>
        <select id="ws-role" bind:value={role}>
          <option value="" disabled>{TEXT.anChoose}</option>
          {#each ROLES as r (r)}
            <option value={r}>
              {(TEXT.wsRoleWords as Record<string, string>)[r]}
            </option>
          {/each}
        </select>
        {#if role !== "nobody"}
          <label for="ws-name">{TEXT.wsName}</label>
          <input id="ws-name" bind:value={name} autocomplete="off" />
        {/if}
        {#if canHoldPower(role)}
          <label for="ws-paper">{TEXT.wsPaper}</label>
          <select id="ws-paper" bind:value={paper}>
            <option value="" disabled>{TEXT.anChoose}</option>
            {#each PAPERS as p (p)}
              <option value={p}>
                {(TEXT.wsPaperWords as Record<string, string>)[p]}
              </option>
            {/each}
          </select>
          {#if seen}
            <label for="ws-seen-by">{TEXT.wsSeenBy}</label>
            <input id="ws-seen-by" bind:value={seenBy} autocomplete="off" />
            <label for="ws-covers">{TEXT.wsCoversLabel}</label>
            <input id="ws-covers" bind:value={covers} autocomplete="off" />
          {/if}
        {/if}
        {#if role !== "nobody" && role !== ""}
          {#each views as v, i (i)}
            <fieldset>
              <legend>{TEXT.wsView.replace("{n}", String(i + 1))}</legend>
              <label for={`ws-about-${i}`}>{TEXT.wsAbout}</label>
              <input
                id={`ws-about-${i}`}
                bind:value={v.about}
                autocomplete="off"
              />
              <label for={`ws-says-${i}`}>{TEXT.wsSays}</label>
              <select id={`ws-says-${i}`} bind:value={v.says}>
                <option value="" disabled>{TEXT.anChoose}</option>
                {#each SAYS as s (s)}
                  <option value={s}>
                    {(TEXT.wsSaysWords as Record<string, string>)[s]}
                  </option>
                {/each}
              </select>
            </fieldset>
          {/each}
          <button
            type="button"
            onclick={() => (views = [...views, { about: "", says: "" }])}
          >
            {TEXT.wsAddView}
          </button>
        {/if}
        <button type="submit" disabled={busy.on}>{TEXT.wsSave}</button>
      </form>
    {/if}
    {#if message}<p role="status">{message}</p>{/if}
  </section>
{/if}
