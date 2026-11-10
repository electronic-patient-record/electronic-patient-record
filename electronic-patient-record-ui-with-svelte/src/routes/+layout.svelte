<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath, type Locale } from "#lib/locales.js";
  import { browser } from "$app/env";
  import { REVIEW, setLocaleSource, TEXT } from "#lib/i18n.js";
  import { STRINGS } from "#lib/strings.js";
  import { setDefaultPurpose } from "#lib/api.js";
  import {
    STAFF_PURPOSES,
    parseStaffPurpose,
    readStoredPurpose,
    storePurpose,
    type StaffPurpose,
  } from "#lib/purpose.js";

  import {
    feedCount,
    referralQueueCount,
    refreshReferralQueueCount,
    worklistCount,
    refreshWorklistCount,
    isWorklistPage,
    deathReportsCount,
    refreshDeathReportsCount,
    isDeathReportsPage,
    isAdminPage,
    refreshFeedCount,
  } from "#lib/feed-count.svelte.js";
  import { idleWarning, lastActivityAt, noteActivity } from "#lib/idle.js";

  let { children } = $props();
  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
  // A machine-written table that no speaker has reviewed says so on every page, in English too,
  // because the warning must not depend on the words it warns about (WL-8, H-051).
  const unreviewed = $derived.by(() => {
    const review = (
      REVIEW as Partial<Record<Locale, { written: string; reviewed: boolean }>>
    )[locale as Locale];
    return review?.written === "machine" && !review.reviewed;
  });
  // In the browser the words follow the URL's locale (`page.data` is reactive, so every template that
  // reads `TEXT` follows a change). The server sets its own source per request (`hooks.server.ts`):
  // setting it here would leak one request's locale into another's.
  if (browser) setLocaleSource(() => page.data.locale);

  // The declared purpose is kept for this tab only. It is read after load, so the server-rendered
  // page and the first client render agree.
  // eslint-disable-next-line svelte/prefer-writable-derived -- read from storage only after hydration, on purpose
  let purpose = $state<StaffPurpose>("operations");
  $effect(() => {
    purpose = readStoredPurpose(window.sessionStorage);
  });
  $effect(() => {
    setDefaultPurpose(purpose);
  });
  function choose(event: Event) {
    const next = parseStaffPurpose(
      (event.currentTarget as HTMLSelectElement).value,
    );
    if (!next) return;
    purpose = next;
    storePurpose(window.sessionStorage, next);
  }
  // The count of refused messages is fetched on the admin pages only.
  const adminPage = $derived(isAdminPage(page.url.pathname));
  $effect(() => {
    if (adminPage) {
      void refreshFeedCount();
      void refreshReferralQueueCount();
    } else {
      feedCount.open = null;
      referralQueueCount.open = null;
    }
  });
  // The count of replies waiting for this person is fetched on the reply list and a patient's record only.
  const deathReportsPage = $derived(isDeathReportsPage(page.url.pathname));
  $effect(() => {
    if (deathReportsPage) void refreshDeathReportsCount();
    else deathReportsCount.open = null;
  });
  const worklistPage = $derived(isWorklistPage(page.url.pathname));
  $effect(() => {
    if (worklistPage) void refreshWorklistCount();
    else worklistCount.open = null;
  });
  // Idle sign-out warning (CQ-8): background looks do not keep the session alive, so warn before the
  // limit and offer one press to stay. Checked every 15 s; the server decides, this only warns.
  const idleSeconds = $derived<number | null>(
    page.data.signIn?.idleSeconds ?? null,
  );
  let minutesLeft = $state<number | null>(null);
  $effect(() => {
    const limit = idleSeconds;
    const tick = () =>
      (minutesLeft = idleWarning(Date.now(), lastActivityAt(), limit));
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

<a class="skip" href="#main">{TEXT.skip}</a>
<header class="site">
  <strong>{TEXT.brand}</strong>
  <nav aria-label="Main">
    <a href={localizePath("/", locale)}>{TEXT.navHome}</a>
    <a href={localizePath("/patients", locale)}>{TEXT.navPatients}</a>
    <a href={localizePath("/patients/new", locale)}>{TEXT.registerLink}</a>
    <a href={localizePath("/death-reports", locale)}
      >{TEXT.navDeathReports}{#if deathReportsPage && deathReportsCount.open}
        <span class="count"
          >({TEXT.drCount.replace("{n}", String(deathReportsCount.open))})</span
        >{/if}</a
    >
    <a href={localizePath("/overdue-reviews", locale)}
      >{TEXT.navOverdueReviews}</a
    >
    <a href={localizePath("/referrals", locale)}
      >{TEXT.navReplies}{#if worklistPage && worklistCount.open}
        <span class="count"
          >({TEXT.wlCount.replace("{n}", String(worklistCount.open))})</span
        >{/if}</a
    >
    <a href={localizePath("/watch-list", locale)}>{TEXT.navWatch}</a>
    <a href={localizePath("/admin/feed", locale)}
      >{TEXT.navFeed}{#if adminPage && feedCount.open}
        <span class="count"
          >({TEXT.feedCount.replace("{n}", String(feedCount.open))})</span
        >{/if}</a
    >
    <a href={localizePath("/admin/referral-queue", locale)}
      >{TEXT.navReferralQueue}{#if adminPage && referralQueueCount.open}
        <span class="count"
          >({TEXT.rqCount.replace(
            "{n}",
            String(referralQueueCount.open),
          )})</span
        >{/if}</a
    >
    <a href={localizePath("/admin/break-glass", locale)}>{TEXT.navGlass}</a>
    <a href={localizePath("/my-record", locale)}>{TEXT.navMyRecord}</a>
  </nav>
  {#if staffPage}
    <label class="purpose">
      {TEXT.purposeLabel}
      <select value={purpose} onchange={choose}>
        {#each STAFF_PURPOSES as p (p)}<option value={p}
            >{TEXT.purposes[p]}</option
          >{/each}
      </select>
    </label>
  {/if}
  {#if page.data.signIn?.enabled}
    {#if page.data.signIn.name}
      <form method="POST" action="/auth/logout" class="account">
        <span>{TEXT.signedInAs} {page.data.signIn.name}</span>
        <button type="submit">{TEXT.signOut}</button>
      </form>
    {:else}
      <a
        class="account"
        data-sveltekit-reload
        href="/auth/login?returnTo={encodeURIComponent(
          page.url.pathname + page.url.search,
        )}">{TEXT.signIn}</a
      >
    {/if}
  {/if}
</header>
<p class="notice" role="note">{TEXT.notClinical}</p>
{#if unreviewed}
  <p class="notice machine" role="note" data-testid="machine-translated">
    <span>{TEXT.machineTranslated}</span>
    <span lang="en">{STRINGS.machineTranslated}</span>
    <a
      lang="en"
      href={localizePath(page.url.pathname + page.url.search, "en-001")}
      >{STRINGS.readInEnglish}</a
    >
  </p>
{/if}
{#if minutesLeft !== null}
  <div class="idle" role="alert">
    {#if minutesLeft > 0}
      <p>{TEXT.idleWarning.replace("{n}", String(minutesLeft))}</p>
      <button type="button" onclick={() => void staySignedIn()}
        >{TEXT.staySignedIn}</button
      >
    {:else}
      <p>{TEXT.idleEnded}</p>
    {/if}
  </div>
{/if}
<main id="main">
  <!-- A new locale remounts the page, so words a page read once when it started are read again. -->
  {#key locale}
    {@render children()}
  {/key}
</main>
