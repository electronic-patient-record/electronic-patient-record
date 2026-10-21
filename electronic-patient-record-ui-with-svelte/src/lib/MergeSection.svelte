<script lang="ts">
  // Merge another record into this one, and undo a merge (ID-5). A merge is a link: nothing is
  // deleted or moved, so undoing it restores both records exactly. The server decides everything
  // (who may, whether they look like the same person); this page shows its reasons.
  import { call, commonProblem } from "./api.js";
  import { STRINGS } from "./strings.js";

  let {
    patientId,
    replaces,
    onchange,
  }: {
    patientId: string;
    replaces: string[];
    onchange: () => void | Promise<void>;
  } = $props();

  let source = $state("");
  let why = $state("");
  let message = $state<string | null>(null);

  async function send(
    path: "merge" | "unmerge",
    sourceId: string,
    done: string,
  ) {
    message = null;
    if (!why.trim()) {
      message = STRINGS.mergeWhy;
      return;
    }
    const res = await call<{ errors?: string[] }>(
      "POST",
      `/api/patients/${patientId}/${path}`,
      { source: sourceId.trim(), reason: why.trim() },
    );
    const common = commonProblem(res.status);
    if (common) message = STRINGS[common];
    else if (res.status === 200 || res.status === 201) {
      message = done;
      source = why = "";
      await onchange();
    } else if (res.status === 404) message = STRINGS.notFound;
    else if (res.status === 422 && res.body?.errors)
      message = res.body.errors.join(" ");
    else message = STRINGS.error;
  }
</script>

<section aria-labelledby="merge-h">
  <h2 id="merge-h">{STRINGS.mergeTitle}</h2>
  <p>{STRINGS.mergeIntro}</p>
  {#if replaces.length > 0}
    <h3>{STRINGS.mergedIn}</h3>
    <ul>
      {#each replaces as r (r)}
        <li>
          <code>{r}</code>
          <button type="button" onclick={() => void send("unmerge", r, STRINGS.mergeUndone)}>
            {STRINGS.mergeUndo}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <label for="m-source">{STRINGS.mergeSource}</label>
  <input id="m-source" bind:value={source} autocomplete="off" />
  <label for="m-why">{STRINGS.mergeWhy}</label>
  <input id="m-why" bind:value={why} autocomplete="off" />
  <button type="button" disabled={!source.trim()} onclick={() => void send("merge", source, STRINGS.mergeDone)}>
    {STRINGS.mergeDo}
  </button>
  {#if message}<p role="status">{message}</p>{/if}
</section>
