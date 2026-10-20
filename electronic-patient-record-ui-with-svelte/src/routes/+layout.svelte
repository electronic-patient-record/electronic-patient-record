<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { DEFAULT_LOCALE, localizePath } from "#lib/locales.js";
  import { STRINGS } from "#lib/strings.js";

  let { children } = $props();
  const locale = $derived(page.data.locale ?? DEFAULT_LOCALE);
</script>

<a class="skip" href="#main">{STRINGS.skip}</a>
<header class="site">
  <strong>{STRINGS.brand}</strong>
  <nav aria-label="Main">
    <a href={localizePath("/", locale)}>{STRINGS.navHome}</a>
    <a href={localizePath("/patients", locale)}>{STRINGS.navPatients}</a>
  </nav>
</header>
<p class="notice" role="note">{STRINGS.notClinical}</p>
<main id="main">
  {@render children()}
</main>
