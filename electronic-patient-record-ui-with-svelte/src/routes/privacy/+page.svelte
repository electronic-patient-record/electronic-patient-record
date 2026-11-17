<script lang="ts">
  // The privacy notice: what this app keeps in the browser, why, and what it does not keep. The rows are
  // `src/lib/privacy.ts`, which a unit test holds to what the code stores.
  import { TEXT } from "#lib/i18n.js";
  import { KEPT } from "#lib/privacy.js";
</script>

<svelte:head><title>{TEXT.privacyTitle}</title></svelte:head>

<h1>{TEXT.privacyTitle}</h1>
<p>{TEXT.privacyIntro}</p>
<p>{TEXT.privacyScope}</p>

<h2>{TEXT.privacyKeptTitle}</h2>
<table aria-label={TEXT.privacyTableLabel}>
  <thead>
    <tr>
      <th scope="col">{TEXT.privacyName}</th>
      <th scope="col">{TEXT.privacyWhere}</th>
      <th scope="col">{TEXT.privacyWhat}</th>
      <th scope="col">{TEXT.privacyWhen}</th>
      <th scope="col">{TEXT.privacyHowLong}</th>
    </tr>
  </thead>
  <tbody>
    {#each KEPT as k (k.name)}
      <tr data-testid="privacy-row">
        <th scope="row"><code>{k.name}</code></th>
        <td>{TEXT.privacyPlaces[k.place]}</td>
        <td>{TEXT.privacyRows[k.row].what}</td>
        <td>
          {TEXT.privacyRows[k.row].when}
          {#if k.signInOnly}<br />{TEXT.privacySignInOnly}{/if}
        </td>
        <td>{TEXT.privacyRows[k.row].kept}</td>
      </tr>
    {/each}
  </tbody>
</table>

<h2>{TEXT.privacyWhyTitle}</h2>
<p>{TEXT.privacyWhy}</p>

<h2>{TEXT.privacyNotKeptTitle}</h2>
<ul>
  {#each TEXT.privacyNotKept as line (line)}<li>{line}</li>{/each}
</ul>

<h2>{TEXT.privacyRemoveTitle}</h2>
<p>{TEXT.privacyRemove}</p>
