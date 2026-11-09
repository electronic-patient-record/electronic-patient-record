<script lang="ts">
  import { call, commonProblem } from "#lib/api.js";
  import type { OverdueReview } from "#lib/adjustments.js";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let items = $state<OverdueReview[] | null>(null);
  let limit = $state(50);
  let more = $state(false);
  let message = $state<string | null>(null);

  async function load() {
    const res = await call<{
      items: OverdueReview[];
      limit: number;
      more: boolean;
    }>("GET", "/api/overdue-reviews");
    const problem = commonProblem(res.status);
    if (problem) {
      items = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      items = res.body.items;
      limit = res.body.limit;
      more = res.body.more;
    } else message = TEXT.error;
  }
  $effect(() => {
    void load();
  });
</script>

<svelte:head><title>{TEXT.orTitle}</title></svelte:head>
<h1>{TEXT.orTitle}</h1>
<p>{TEXT.orIntro}</p>
<p>{TEXT.orAccountable}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if items !== null}
  {#if items.length === 0}
    <p>{TEXT.orNone}</p>
  {:else}
    <p>{TEXT.orCount.replace("{n}", String(items.length))}</p>
    {#if more}
      <p>{TEXT.orMore.replace("{n}", String(limit))}</p>
    {/if}
    <ul>
      {#each items as i (i.adjustment_id)}
        <li>
          <strong>{TEXT.orDue.replace("{day}", i.review_by)}</strong>
          {#if i.contact_held}<p>{TEXT.orHeld}</p>{/if}
          <a
            href={localizePath(
              `/patients/${i.patient_id}#adjustments-h`,
              locale,
            )}>{TEXT.orOpen}</a
          >
        </li>
      {/each}
    </ul>
  {/if}
{/if}
