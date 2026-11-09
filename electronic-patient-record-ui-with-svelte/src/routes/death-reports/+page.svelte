<script lang="ts">
  import { call, commonProblem } from "#lib/api.js";
  import { siteWhenText } from "#lib/access-log.js";
  import type { WaitingReport } from "#lib/death.js";
  import { refreshDeathReportsCount } from "#lib/feed-count.svelte.js";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  let items = $state<WaitingReport[] | null>(null);
  let limit = $state(50);
  let more = $state(false);
  let message = $state<string | null>(null);
  let siteZone = $state<string | undefined>(undefined);

  async function load() {
    const res = await call<{
      items: WaitingReport[];
      limit: number;
      more: boolean;
      site_time_zone?: string;
    }>("GET", "/api/death-reports");
    const problem = commonProblem(res.status);
    if (problem) {
      items = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      items = res.body.items;
      limit = res.body.limit;
      more = res.body.more;
      siteZone = res.body.site_time_zone;
      void refreshDeathReportsCount();
    } else message = TEXT.error;
  }
  $effect(() => {
    void load();
  });
</script>

<svelte:head><title>{TEXT.drTitle}</title></svelte:head>
<h1>{TEXT.drTitle}</h1>
<p>{TEXT.drIntro}</p>
<p>{TEXT.drAccountable}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if items !== null}
  {#if items.length === 0}
    <p>{TEXT.drNone}</p>
  {:else}
    <p>{TEXT.drCount.replace("{n}", String(items.length))}</p>
    {#if more}
      <p>{TEXT.drMore.replace("{n}", String(limit))}</p>
    {/if}
    <ul>
      {#each items as i (i.assertion_id)}
        <li>
          <strong
            >{TEXT.drReported.replace(
              "{when}",
              siteWhenText(i.reported_at, siteZone),
            )}</strong
          >
          <a href={localizePath(`/patients/${i.patient_id}`, locale)}
            >{TEXT.drOpen}</a
          >
        </li>
      {/each}
    </ul>
  {/if}
{/if}
