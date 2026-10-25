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
  import { idleWarning, lastActivityAt, noteActivity } from "#lib/idle.js";

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
  // Idle sign-out warning (CQ-8): background looks do not keep the session alive, so warn before the
  // limit and offer one press to stay. Checked every 15 s; the server decides, this only warns.
  const idleSeconds = $derived<number | null>(page.data.signIn?.idleSeconds ?? null);
  let minutesLeft = $state<number | null>(null);
  $effect(() => {
    const limit = idleSeconds;
    const tick = () => (minutesLeft = idleWarning(Date.now(), lastActivityAt(), limit));
    tick();
    const timer = setInterval(tick, 15_000);
    return () => clearInterval(timer);
  });
  async function staySignedIn() {
    try {
      const res = await fetch("/auth/keepalive", { method: "POST" });
      if (res.ok) {
        noteActivity();
        minutesLeft = null;
      } else {
        minutesLeft = 0;
      }
    } catch {
      minutesLeft = 0;
    }
  }

  // "My record" always declares patient access, so the staff choice does not apply there.
  const staffPage = $derived(!page.url.pathname.includes("/my-record"));

  // Set once the page has hydrated, so a browser test waits for the handlers to be attached before it
  // clicks (a click before that is a native form submit that does nothing).
  $effect(() => {
    document.documentElement.setAttribute("data-hydrated", "");
  });
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
    <a href={localizePath("/admin/break-glass", locale)}>{STRINGS.navGlass}</a>
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
{#if minutesLeft !== null}
  <div class="idle" role="alert">
    {#if minutesLeft > 0}
      <p>{STRINGS.idleWarning.replace("{n}", String(minutesLeft))}</p>
      <button type="button" onclick={() => void staySignedIn()}>{STRINGS.staySignedIn}</button>
    {:else}
      <p>{STRINGS.idleEnded}</p>
    {/if}
  </div>
{/if}
<main id="main">
  {@render children()}
</main>
