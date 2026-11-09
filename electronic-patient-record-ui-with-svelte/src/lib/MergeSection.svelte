<script lang="ts">
  // Merge another record into this one, and undo a merge (ID-5). A merge is a link: nothing is
  // deleted or moved, so undoing it restores both records exactly. The server decides everything
  // (who may, whether they look like the same person); this page shows its reasons.
  import { page } from "$app/state";
  import { call, commonProblem } from "./api.js";
  import {
    notSamePersonText,
    refusalText,
    type ConflictCode,
  } from "./conflicts.js";
  import { DEFAULT_LOCALE } from "./locales.js";
  import { TEXT } from "./i18n.js";

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
  let busy = $state(false);

  async function send(
    path: "merge" | "unmerge",
    sourceId: string,
    done: string,
  ) {
    if (busy) return;
    message = null;
    if (!why.trim()) {
      message = TEXT.mergeWhy;
      return;
    }
    busy = true;
    let res;
    try {
      res = await call<{
        errors?: string[];
        conflict_codes?: ConflictCode[];
        refusal?: string;
        refusal_into?: string;
      }>("POST", `/api/patients/${patientId}/${path}`, {
        source: sourceId.trim(),
        reason: why.trim(),
      });
    } finally {
      busy = false;
    }
    const common = commonProblem(res.status);
    if (common) message = TEXT[common];
    else if (res.status === 200 || res.status === 201) {
      message = done;
      source = why = "";
      await onchange();
    } else if (res.status === 404) message = TEXT.notFound;
    else if (res.status === 422 && res.body?.errors) {
      const locale = page.data.locale ?? DEFAULT_LOCALE;
      message =
        notSamePersonText(res.body.conflict_codes, locale) ??
        refusalText(res.body.refusal, res.body.refusal_into, locale) ??
        res.body.errors.join(" ");
    } else message = TEXT.error;
  }
</script>

<section aria-labelledby="merge-h">
  <h2 id="merge-h">{TEXT.mergeTitle}</h2>
  <p>{TEXT.mergeIntro}</p>
  {#if replaces.length > 0}
    <h3>{TEXT.mergedIn}</h3>
    <ul>
      {#each replaces as r (r)}
        <li>
          <code>{r}</code>
          <button
            type="button"
            disabled={busy}
            onclick={() => void send("unmerge", r, TEXT.mergeUndone)}
          >
            {TEXT.mergeUndo}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <label for="m-source">{TEXT.mergeSource}</label>
  <input id="m-source" bind:value={source} autocomplete="off" />
  <label for="m-why">{TEXT.mergeWhy}</label>
  <input id="m-why" bind:value={why} autocomplete="off" />
  <button
    type="button"
    disabled={busy || !source.trim()}
    onclick={() => void send("merge", source, TEXT.mergeDone)}
  >
    {TEXT.mergeDo}
  </button>
  {#if message}<p role="status">{message}</p>{/if}
</section>
