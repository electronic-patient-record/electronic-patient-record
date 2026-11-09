<script lang="ts">
  import { Busy } from "#lib/busy.svelte.js";
  import { call, commonProblem } from "#lib/api.js";
  import {
    acknowledgeBody,
    itemHeadline,
    untimedWarning,
    type Untimed,
    type WorkItem,
  } from "#lib/referral-worklist.js";
  import { refreshWorklistCount } from "#lib/feed-count.svelte.js";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { TEXT } from "#lib/i18n.js";

  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  const busy = new Busy();
  let items = $state<WorkItem[] | null>(null);
  let limit = $state(50);
  // The service says when the list was cut short (PR-1); an older one says nothing, so a full list counts as cut.
  let more = $state(false);
  let message = $state<string | null>(null);
  let untimed = $state<Untimed | undefined>(undefined);
  let siteZone = $state<string | undefined>(undefined);

  async function load() {
    const res = await call<{
      items: WorkItem[];
      limit: number;
      more?: boolean;
      untimed?: Untimed;
      site_time_zone?: string;
    }>("GET", "/api/referrals/worklist");
    const problem = commonProblem(res.status);
    if (problem) {
      items = null;
      message = TEXT[problem];
    } else if (res.status === 200) {
      items = res.body.items;
      limit = res.body.limit;
      more = res.body.more ?? items.length >= limit;
      untimed = res.body.untimed;
      siteZone = res.body.site_time_zone;
      void refreshWorklistCount();
    } else message = TEXT.error;
  }
  $effect(() => {
    void load();
  });

  async function acknowledge(i: WorkItem) {
    await busy.run(async () => {
      const res = await call(
        "POST",
        `/api/referrals/${i.referral_id}/acknowledge`,
        acknowledgeBody(i),
      );
      const problem = commonProblem(res.status);
      if (problem) message = TEXT[problem];
      else if (res.status === 200) message = TEXT.wlSeen;
      // 409: a newer answer arrived, or it was already seen. Reload to show what is true now.
      else if (res.status === 409) message = TEXT.wlChanged;
      else message = TEXT.error;
      if (!problem && (res.status === 200 || res.status === 409)) await load();
    });
  }
</script>

<svelte:head><title>{TEXT.wlTitle}</title></svelte:head>
<h1>{TEXT.wlTitle}</h1>
<p>{TEXT.wlIntro}</p>
{#if message}<p role="status">{message}</p>{/if}
{#if untimedWarning(untimed)}<p role="status">{untimedWarning(untimed)}</p>{/if}
{#if items !== null}
  {#if items.length === 0}
    <p>{TEXT.wlNone}</p>
  {:else}
    <p>{TEXT.wlCount.replace("{n}", String(items.length))}</p>
    {#if more}
      <p>{TEXT.wlMore.replace("{n}", String(limit))}</p>
    {/if}
    <ul>
      {#each items as i (i.referral_id)}
        <li>
          <strong>{itemHeadline(i, siteZone)}</strong>
          <a href={localizePath(`/patients/${i.patient_id}`, locale)}
            >{TEXT.wlOpen}</a
          >
          {#if i.why === "reply"}
            <button
              disabled={busy.on}
              type="button"
              onclick={() => void acknowledge(i)}>{TEXT.wlAcknowledge}</button
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
