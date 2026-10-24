<script lang="ts">
  import { page } from "$app/state";
  import { call, commonProblem } from "#lib/api.js";
  import {
    describeConsent,
    describeProxy,
    emptyConsentForm,
    formToConsent,
    LABELS,
    PROXY_BASES,
    PROXY_SCOPES,
    proxyTerms,
    PURPOSES,
    RELATIONSHIP_KINDS,
    relationshipRecord,
    RESOURCES,
    ROLES,
    type ProxyBasis,
    type ProxyScope,
    type RelationshipKind,
    type StoredConsent,
    type StoredRelationship,
  } from "#lib/access.js";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { STRINGS } from "#lib/strings.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const id = $derived(page.params.id);

  let rels = $state<StoredRelationship[] | null>(null);
  let relMessage = $state<string | null>(null);
  let consents = $state<StoredConsent[] | null>(null);
  let consentMessage = $state<string | null>(null);

  let subject = $state("");
  let kind = $state<RelationshipKind>("care-team");
  let relStart = $state("");
  let relEnd = $state("");
  let relWhy = $state("");
  let basis = $state<ProxyBasis>("parental-responsibility");
  let proxyScopes = $state<ProxyScope[]>([]);
  let lastDay = $state("");
  let cform = $state(emptyConsentForm());
  let consentWhy = $state("");

  async function loadRels() {
    const res = await call<{ relationships: StoredRelationship[] }>(
      "GET",
      `/api/patients/${id}/relationships`,
    );
    const problem = commonProblem(res.status);
    if (problem) {
      rels = null;
      relMessage = STRINGS[problem];
    } else if (res.status === 200) {
      rels = res.body.relationships;
    } else relMessage = res.status === 404 ? STRINGS.notFound : STRINGS.error;
  }

  async function loadConsents() {
    const res = await call<{ consents: StoredConsent[] }>(
      "GET",
      `/api/patients/${id}/consents`,
    );
    const problem = commonProblem(res.status);
    if (problem) {
      consents = null;
      consentMessage = STRINGS[problem];
    } else if (res.status === 200) {
      consents = res.body.consents;
    } else
      consentMessage = res.status === 404 ? STRINGS.notFound : STRINGS.error;
  }

  $effect(() => {
    void id;
    void loadRels();
    void loadConsents();
  });

  /** Show what a write answered, and reload the list it changed. */
  async function settle(
    res: { status: number; body: unknown },
    set: (m: string | null) => void,
    reload: () => Promise<void>,
  ) {
    const common = commonProblem(res.status);
    const body = res.body as { errors?: string[] } | null;
    if (common) set(STRINGS[common]);
    else if (res.status === 200 || res.status === 201) {
      set(STRINGS.accessSaved);
      await reload();
    } else if (res.status === 409) {
      set(STRINGS.accessChanged);
      await reload();
    } else if (res.status === 422 && body?.errors) set(body.errors.join(" "));
    else set(STRINGS.error);
  }

  async function addRelationship(event: SubmitEvent) {
    event.preventDefault();
    relMessage = null;
    const res = await call("POST", `/api/patients/${id}/relationships`, {
      relationship: relationshipRecord(
        subject,
        kind,
        relStart,
        relEnd,
        proxyTerms(basis, proxyScopes, lastDay),
      ),
      reason: relWhy.trim() || undefined,
    });
    if (res.status === 201) {
      subject = relStart = relEnd = relWhy = lastDay = "";
      proxyScopes = [];
    }
    await settle(res, (m) => (relMessage = m), loadRels);
  }

  async function endRelationship(r: StoredRelationship) {
    relMessage = null;
    await settle(
      await call("PUT", `/api/patients/${id}/relationships/${r.id}`, {
        relationship: { ...r.relationship, status: "ended" },
        expected_version: r.version,
        reason: relWhy.trim() || undefined,
      }),
      (m) => (relMessage = m),
      loadRels,
    );
  }

  async function addConsent(event: SubmitEvent) {
    event.preventDefault();
    consentMessage = null;
    const res = await call("POST", `/api/patients/${id}/consents`, {
      consent: formToConsent(cform),
      reason: consentWhy.trim() || undefined,
    });
    if (res.status === 201) {
      cform = emptyConsentForm();
      consentWhy = "";
    }
    await settle(res, (m) => (consentMessage = m), loadConsents);
  }

  async function changeConsent(
    c: StoredConsent,
    status: "inactive" | "entered-in-error",
  ) {
    consentMessage = null;
    await settle(
      await call("PUT", `/api/patients/${id}/consents/${c.id}`, {
        consent: { ...c.consent, status },
        expected_version: c.version,
        reason: consentWhy.trim() || undefined,
      }),
      (m) => (consentMessage = m),
      loadConsents,
    );
  }

  const choices = $derived([
    [STRINGS.consentRoles, ROLES, "actors"],
    [STRINGS.consentPurposes, PURPOSES, "purposes"],
    [STRINGS.consentResources, RESOURCES, "resources"],
    [STRINGS.consentLabels, LABELS, "labels"],
  ] as const);

  function toggle(field: "actors" | "purposes" | "resources" | "labels", value: string, on: boolean) {
    const now = cform[field];
    cform[field] = on ? [...now, value] : now.filter((v) => v !== value);
  }
</script>

<svelte:head><title>{STRINGS.accessTitle}</title></svelte:head>

<h1>{STRINGS.accessTitle}</h1>
<p>
  <a href={localizePath(`/patients/${id}`, locale)}>{STRINGS.accessBack}</a>
</p>
<p>{STRINGS.accessIntro}</p>

<section aria-labelledby="rel-h">
  <h2 id="rel-h">{STRINGS.relTitle}</h2>
  {#if rels !== null}
    {#if rels.length === 0}
      <p>{STRINGS.relNone}</p>
    {:else}
      <ul>
        {#each rels as r (r.id)}
          <li>
            <strong>{STRINGS.relKinds[r.relationship.kind]}</strong>
            · <code>{r.relationship.subject}</code>
            · {r.relationship.status === "active" ? STRINGS.relActive : STRINGS.relEnded}
            {#if r.relationship.period?.start || r.relationship.period?.end}
              · {r.relationship.period.start ?? "…"} – {r.relationship.period.end ?? "…"}
            {/if}
            {#if r.relationship.kind === "proxy"}
              · {describeProxy(r.relationship.proxy)}
            {/if}
            {#if r.relationship.status === "active"}
              <button type="button" onclick={() => void endRelationship(r)}>
                {STRINGS.relEnd_}
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    <form onsubmit={addRelationship}>
      <label for="r-subject">{STRINGS.relSubject}</label>
      <input id="r-subject" bind:value={subject} required autocomplete="off" />
      <label for="r-kind">{STRINGS.relKind}</label>
      <select id="r-kind" bind:value={kind}>
        {#each RELATIONSHIP_KINDS as k (k)}
          <option value={k}>{STRINGS.relKinds[k]}</option>
        {/each}
      </select>
      {#if kind === "proxy"}
        <label for="r-basis">{STRINGS.proxyBasis}</label>
        <select id="r-basis" bind:value={basis}>
          {#each PROXY_BASES as b (b)}
            <option value={b}>{STRINGS.proxyBases[b]}</option>
          {/each}
        </select>
        <fieldset>
          <legend>{STRINGS.proxyScopesLegend}</legend>
          {#each PROXY_SCOPES as s (s)}
            <label>
              <input
                type="checkbox"
                checked={proxyScopes.includes(s)}
                onchange={(e) =>
                  (proxyScopes = e.currentTarget.checked
                    ? [...proxyScopes, s]
                    : proxyScopes.filter((v) => v !== s))}
              />
              {STRINGS.proxyScopes[s]}
            </label>
          {/each}
        </fieldset>
        <label for="r-last">{STRINGS.proxyLastDay}</label>
        <input id="r-last" bind:value={lastDay} autocomplete="off" />
      {/if}
      <label for="r-start">{STRINGS.relStart}</label>
      <input id="r-start" bind:value={relStart} autocomplete="off" />
      <label for="r-end">{STRINGS.relEnd}</label>
      <input id="r-end" bind:value={relEnd} autocomplete="off" />
      <label for="r-why">{STRINGS.relWhy}</label>
      <input id="r-why" bind:value={relWhy} autocomplete="off" />
      <button type="submit">{STRINGS.relAdd}</button>
    </form>
  {/if}
  {#if relMessage}<p role="status">{relMessage}</p>{/if}
</section>

<section aria-labelledby="consent-h">
  <h2 id="consent-h">{STRINGS.consentTitle}</h2>
  {#if consents !== null}
    {#if consents.length === 0}
      <p>{STRINGS.consentNone}</p>
    {:else}
      <ul>
        {#each consents as c (c.id)}
          <li>
            <strong>{STRINGS.consentStatuses[c.consent.status]}</strong>
            · {describeConsent(c.consent)}
            {#if c.consent.status === "active"}
              <button type="button" onclick={() => void changeConsent(c, "inactive")}>
                {STRINGS.consentWithdraw}
              </button>
            {/if}
            {#if c.consent.status !== "entered-in-error"}
              <button type="button" onclick={() => void changeConsent(c, "entered-in-error")}>
                {STRINGS.consentMistake}
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    <form onsubmit={addConsent}>
      <label>
        <input type="checkbox" bind:checked={cform.nationalOptOut} />
        {STRINGS.consentOptOutBox}
      </label>
      {#if !cform.nationalOptOut}
        <label for="c-effect">{STRINGS.consentEffect}</label>
        <select id="c-effect" bind:value={cform.effect}>
          <option value="deny">{STRINGS.consentDeny}</option>
          <option value="permit">{STRINGS.consentPermit}</option>
        </select>
        {#each choices as [legend, values, field] (field)}
          <fieldset>
            <legend>{legend}</legend>
            {#each values as v (v)}
              <label>
                <input
                  type="checkbox"
                  checked={cform[field].includes(v)}
                  onchange={(e) => toggle(field, v, e.currentTarget.checked)}
                />
                {v}
              </label>
            {/each}
          </fieldset>
        {/each}
        <label for="c-start">{STRINGS.consentFrom}</label>
        <input id="c-start" bind:value={cform.start} autocomplete="off" />
        <label for="c-end">{STRINGS.consentUntil}</label>
        <input id="c-end" bind:value={cform.end} autocomplete="off" />
      {/if}
      <label for="c-why">{STRINGS.consentWhy}</label>
      <input id="c-why" bind:value={consentWhy} autocomplete="off" />
      <button type="submit">{STRINGS.consentAdd}</button>
    </form>
  {/if}
  {#if consentMessage}<p role="status">{consentMessage}</p>{/if}
</section>
