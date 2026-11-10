<script lang="ts">
  // My watch list (WL-6): the reader's own active entries, oldest first, as the service sends them
  // (`GET /api/watch-list`): which patient, since when, until when. The list carries no reason, so a
  // reason is read on the patient's page, where it is audited as a read of that record.
  import { call, commonProblem } from "#lib/api.js";
  import { siteWhenText } from "#lib/access-log.js";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { TEXT } from "#lib/i18n.js";

  type Pointer = {
    id: string;
    version: number;
    patient_id: string;
    since: string;
    until: string | null;
  };
  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let items = $state<Pointer[] | null>(null);
  let lapsed = $state<Pointer[]>([]);
  let lapsedMore = $state(false);
  let more = $state(false);
  let limit = $state(200);
  let message = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      const res = await call<{
        items: Pointer[];
        limit: number;
        more: boolean;
        lapsed?: Pointer[];
        lapsed_more?: boolean;
      }>("GET", "/api/watch-list");
      const problem = commonProblem(res.status);
      if (problem) message = TEXT[problem];
      else if (res.status === 200) {
        items = res.body.items;
        more = res.body.more;
        limit = res.body.limit;
        lapsed = res.body.lapsed ?? [];
        lapsedMore = res.body.lapsed_more ?? false;
      } else message = TEXT.error;
    })();
  });
</script>

<svelte:head><title>{TEXT.myWatchTitle}</title></svelte:head>
<h1>{TEXT.myWatchTitle}</h1>
<p>{TEXT.watchNote}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if lapsed.length > 0}
  <section aria-labelledby="lapsed-h">
    <h2 id="lapsed-h">{TEXT.myWatchLapsedTitle}</h2>
    <p role="note">{TEXT.myWatchLapsedNote}</p>
    {#if lapsedMore}<p>{TEXT.myWatchMore.replace("{n}", String(limit))}</p>{/if}
    <ul>
      {#each lapsed as i (i.id)}
        <li>
          {TEXT.myWatchLapsedOn.replace("{until}", i.until ?? "")}
          <a href={localizePath(`/patients/${i.patient_id}`, locale)}
            >{TEXT.myWatchLapsedOpen}</a
          >
        </li>
      {/each}
    </ul>
  </section>
{/if}
{#if items !== null}
  {#if items.length === 0}
    <p>{TEXT.myWatchNone}</p>
  {:else}
    <p>{TEXT.myWatchCount.replace("{n}", String(items.length))}</p>
    {#if more}<p>{TEXT.myWatchMore.replace("{n}", String(limit))}</p>{/if}
    <ul>
      {#each items as i (i.id)}
        <li>
          {TEXT.myWatchSince.replace("{since}", siteWhenText(i.since))}
          {#if i.until}· {TEXT.watchUntil.replace("{until}", i.until)}{/if}
          <a href={localizePath(`/patients/${i.patient_id}`, locale)}
            >{TEXT.myWatchOpen}</a
          >
        </li>
      {/each}
    </ul>
  {/if}
{/if}
