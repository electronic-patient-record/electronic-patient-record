<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { STRINGS } from "#lib/strings.js";
  import { setDefaultPurpose } from "#lib/api.js";
  import {
    STAFF_PURPOSES,
    parseStaffPurpose,
    readStoredPurpose,
    storePurpose,
    type StaffPurpose,
  } from "#lib/purpose.js";

  import { feedCount, isAdminPage, refreshFeedCount } from "#lib/feed-count.svelte.js";

  let { children } = $props();
  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);

  // The declared purpose is kept for this tab only. It is read after load, so the server-rendered
  // page and the first client render agree.
  let purpose = $state<StaffPurpose>("operations");
  $effect(() => {
    purpose = readStoredPurpose(window.sessionStorage);
  });
  $effect(() => {
    setDefaultPurpose(purpose);
  });
  function choose(event: Event) {
    const next = parseStaffPurpose((event.currentTarget as HTMLSelectElement).value);
    if (!next) return;
    purpose = next;
    storePurpose(window.sessionStorage, next);
  }
  // The count of refused messages is fetched on the admin pages only.
  const adminPage = $derived(isAdminPage(page.url.pathname));
  $effect(() => {
    if (adminPage) void refreshFeedCount();
    else feedCount.open = null;
  });
  // "My record" always declares patient access, so the staff choice does not apply there.
  const staffPage = $derived(!page.url.pathname.includes("/my-record"));
</script>

<a class="skip" href="#main">{STRINGS.skip}</a>
<header class="site">
  <strong>{STRINGS.brand}</strong>
  <nav aria-label="Main">
    <a href={localizePath("/", locale)}>{STRINGS.navHome}</a>
    <a href={localizePath("/patients", locale)}>{STRINGS.navPatients}</a>
    <a href={localizePath("/patients/new", locale)}>{STRINGS.registerLink}</a>
    <a href={localizePath("/admin/feed", locale)}>{STRINGS.navFeed}{#if adminPage && feedCount.open}
        <span class="count">({STRINGS.feedCount.replace("{n}", String(feedCount.open))})</span>{/if}</a>
    <a href={localizePath("/my-record", locale)}>{STRINGS.navMyRecord}</a>
  </nav>
  {#if staffPage}
    <label class="purpose">
      {STRINGS.purposeLabel}
      <select value={purpose} onchange={choose}>
        {#each STAFF_PURPOSES as p (p)}<option value={p}>{STRINGS.purposes[p]}</option>{/each}
      </select>
    </label>
  {/if}
  {#if page.data.signIn?.enabled}
    {#if page.data.signIn.name}
      <form method="POST" action="/auth/logout" class="account">
        <span>{STRINGS.signedInAs} {page.data.signIn.name}</span>
        <button type="submit">{STRINGS.signOut}</button>
      </form>
    {:else}
      <a
        class="account"
        data-sveltekit-reload
        href="/auth/login?returnTo={encodeURIComponent(page.url.pathname + page.url.search)}"
        >{STRINGS.signIn}</a
      >
    {/if}
  {/if}
</header>
<p class="notice" role="note">{STRINGS.notClinical}</p>
<main id="main">
  {@render children()}
</main>
