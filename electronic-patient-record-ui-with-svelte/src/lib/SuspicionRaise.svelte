<script lang="ts">
  // Record a suspicion on a referral (RS-1, RS-8). The pathways are the service's newest versions; the
  // form offers only what the chosen pathway asks for (a band, its factors, its subtype list, the
  // person's agreement) and defaults nothing a clinician must decide: whether the patient was told has
  // no preselected answer. The service checks every rule again and its refusal is shown as it gives it.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import {
    EMPTY_FORM,
    buildSuspicion,
    type Pathway,
    type RaiseForm,
  } from "./suspicions.js";
  import { TEXT } from "./i18n.js";

  let {
    patientId,
    referralId,
    pathways,
    done,
  }: {
    patientId: string;
    referralId: string;
    pathways: Pathway[];
    done: () => void;
  } = $props();

  const busy = new Busy();
  let f = $state<RaiseForm>({ ...EMPTY_FORM, factors: [] });
  let message = $state<string | null>(null);
  const chosen = $derived(pathways.find((p) => p.id === f.pathway));
  const id = (name: string) => `raise-${name}-${referralId}`;

  async function send(event: SubmitEvent) {
    event.preventDefault();
    const built = buildSuspicion(f, pathways);
    if (built.problems.length > 0) {
      message = TEXT.raiseMissing.replace(
        "{fields}",
        built.problems.map((p) => TEXT.raiseProblems[p]).join(", "),
      );
      return;
    }
    await busy.run(async () => {
      message = null;
      const res = await call(
        "POST",
        `/api/patients/${patientId}/referrals/${referralId}/suspicion`,
        built.body,
      );
      const common = commonProblem(res.status);
      const body = res.body as { errors?: string[] } | null;
      if (common) message = TEXT[common];
      else if (res.status === 201) {
        message = TEXT.raiseDone;
        done();
      } else if (res.status === 409) {
        message = TEXT.referralChanged;
        done();
      } else if (res.status === 422 && body?.errors)
        message = body.errors.join(" ");
      else message = TEXT.error;
    });
  }
</script>

<details>
  <summary>{TEXT.raiseTitle}</summary>
  <form onsubmit={send} novalidate>
    <label for={id("pathway")}>{TEXT.raisePathway}</label>
    <select id={id("pathway")} bind:value={f.pathway}>
      <option value="">{TEXT.raiseChoose}</option>
      {#each pathways as p (p.id)}
        <option value={p.id}
          >{p.title}{p.approved_by.trim() ? "" : ` ${TEXT.raiseDraft}`}</option
        >
      {/each}
    </select>
    <label for={id("condition")}>{TEXT.raiseCondition}</label>
    <input
      id={id("condition")}
      bind:value={f.condition}
      maxlength="200"
      autocomplete="off"
    />
    <label for={id("point")}>{TEXT.raisePoint}</label>
    <input id={id("point")} type="datetime-local" bind:value={f.point} />
    <label for={id("source")}>{TEXT.raiseSource}</label>
    <select id={id("source")} bind:value={f.source}>
      <option value="">{TEXT.raiseChoose}</option>
      {#each Object.entries(TEXT.raiseSources) as [v, words] (v)}
        <option value={v}>{words}</option>
      {/each}
    </select>
    <label for={id("by")}>{TEXT.raiseBy}</label>
    <input id={id("by")} bind:value={f.by} maxlength="200" autocomplete="off" />
    <label for={id("grounds")}>{TEXT.raiseGroundsKind}</label>
    <select id={id("grounds")} bind:value={f.groundsKind}>
      <option value="">{TEXT.raiseChoose}</option>
      {#each Object.entries(TEXT.raiseGroundsKinds) as [v, words] (v)}
        <option value={v}>{words}</option>
      {/each}
    </select>
    <label for={id("detail")}>{TEXT.raiseGroundsDetail}</label>
    <input
      id={id("detail")}
      bind:value={f.groundsDetail}
      maxlength="500"
      autocomplete="off"
    />
    {#if f.groundsKind === "result"}
      <label for={id("result")}>{TEXT.raiseResultId}</label>
      <input id={id("result")} bind:value={f.resultId} autocomplete="off" />
    {/if}
    <fieldset>
      <legend>{TEXT.raiseTold}</legend>
      <label
        ><input type="radio" value="told" bind:group={f.told} />
        {TEXT.raiseToldYes}</label
      >
      <label
        ><input type="radio" value="not-told" bind:group={f.told} />
        {TEXT.raiseToldNo}</label
      >
    </fieldset>
    {#if chosen && chosen.band.length > 0}
      <label for={id("band")}>{TEXT.raiseBand}</label>
      <select id={id("band")} bind:value={f.band}>
        <option value="">{TEXT.raiseChoose}</option>
        {#each chosen.band as b (b.id)}
          <option value={b.id}>{b.text}</option>
        {/each}
      </select>
    {/if}
    {#if chosen && chosen.factor.length > 0}
      <fieldset>
        <legend>{TEXT.raiseFactors}</legend>
        {#each chosen.factor as x (x.id)}
          <label
            ><input type="checkbox" value={x.id} bind:group={f.factors} />
            {x.text}</label
          >
        {/each}
      </fieldset>
    {/if}
    {#if chosen?.subtypes}
      <label for={id("subtype")}>{chosen.subtypes.title}</label>
      <select id={id("subtype")} bind:value={f.subtype}>
        <option value=""
          >{chosen.subtypes.required
            ? TEXT.raiseChoose
            : TEXT.raiseSubtypeNone}</option
        >
        {#each chosen.subtypes.code as c (c.code)}
          <option value={c.code}>{c.name}</option>
        {/each}
      </select>
    {/if}
    {#if chosen?.needs_agreement}
      <fieldset>
        <legend>{TEXT.raiseAgreed}</legend>
        <label
          ><input type="radio" value="yes" bind:group={f.agreed} />
          {TEXT.raiseAgreedYes}</label
        >
        <label
          ><input type="radio" value="no" bind:group={f.agreed} />
          {TEXT.raiseAgreedNo}</label
        >
      </fieldset>
    {/if}
    <label for={id("instruction")}>{TEXT.raiseInstruction}</label>
    <input
      id={id("instruction")}
      bind:value={f.instruction}
      maxlength="500"
      autocomplete="off"
    />
    <button disabled={busy.on} type="submit">{TEXT.raiseSend}</button>
  </form>
  {#if message}<p role="status">{message}</p>{/if}
</details>
