<script lang="ts">
  // Correct an open suspicion (RS-1, RS-8): whether the patient was told, the point of suspicion, the
  // instruction, with a reason. The form starts from what is recorded; the rest of the suspicion is sent
  // as it is, against the version on screen (a 409 reloads). The earlier version is kept by the service.
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "./api.js";
  import { buildAmendment, localInput, type AmendForm } from "./suspicions.js";
  import { TEXT } from "./i18n.js";

  let {
    patientId,
    referralId,
    version,
    current,
    done,
  }: {
    patientId: string;
    referralId: string;
    version: number;
    current: Record<string, unknown>;
    done: () => void;
  } = $props();

  const busy = new Busy();
  const start = (): AmendForm => ({
    told: current.patient_told === "told" ? "told" : "not-told",
    point: localInput(String(current.point_of_suspicion ?? "")),
    instruction: String(current.instruction ?? ""),
    reason: "",
  });
  let f = $state<AmendForm>(start());
  let message = $state<string | null>(null);
  const id = (name: string) => `amend-${name}-${referralId}`;

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const built = buildAmendment(current, version, f);
    if (built.problems.length > 0) {
      message = TEXT.amendSuspicionMissing.replace(
        "{problems}",
        built.problems.map((p) => TEXT.amendSuspicionProblems[p]).join(" "),
      );
      return;
    }
    await busy.run(async () => {
      message = null;
      const res = await call(
        "PUT",
        `/api/patients/${patientId}/referrals/${referralId}/suspicion`,
        built.body,
      );
      const common = commonProblem(res.status);
      const body = res.body as { errors?: string[] } | null;
      if (common) message = TEXT[common];
      else if (res.status === 200) {
        message = TEXT.amendSuspicionDone;
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
  <summary>{TEXT.amendSuspicionTitle}</summary>
  <p>{TEXT.amendSuspicionNote}</p>
  <form onsubmit={save} novalidate>
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
    <label for={id("point")}>{TEXT.raisePoint}</label>
    <input id={id("point")} type="datetime-local" bind:value={f.point} />
    <label for={id("instruction")}>{TEXT.raiseInstruction}</label>
    <input
      id={id("instruction")}
      bind:value={f.instruction}
      maxlength="500"
      autocomplete="off"
    />
    <label for={id("reason")}>{TEXT.amendSuspicionReason}</label>
    <input
      id={id("reason")}
      bind:value={f.reason}
      maxlength="500"
      autocomplete="off"
    />
    <button disabled={busy.on} type="submit">{TEXT.amendSuspicionSend}</button>
  </form>
  {#if message}<p role="status">{message}</p>{/if}
</details>
