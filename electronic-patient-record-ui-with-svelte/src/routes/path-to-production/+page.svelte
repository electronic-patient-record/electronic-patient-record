<script lang="ts">
  // The path to production (PR-7): what production means, and one line per area of the evidence
  // checklist (`governance/production-readiness.md`) with a readiness estimate on a Lily slider. The
  // slider shows the estimate and is disabled: the reader cannot move it, because an estimate moved
  // in a browser would be a number nobody wrote. The percentage is also given as text.
  import { Slider } from "@lilydesignsystem/svelte-headless";
  import { TEXT } from "#lib/i18n.js";
  import {
    ESTIMATED_ON,
    PRODUCTION_STEPS,
    plain,
  } from "#lib/path-to-production.js";
</script>

<svelte:head><title>{TEXT.ptpTitle}</title></svelte:head>

<h1>{TEXT.ptpTitle}</h1>
<p>{TEXT.ptpMeaning}</p>
<p>{TEXT.ptpNotThere}</p>
<p>{TEXT.ptpEstimates.replace("{day}", ESTIMATED_ON)}</p>
<p>{TEXT.ptpNoTotal}</p>
<p>{TEXT.ptpEnglish}</p>

<ol class="steps" aria-label={TEXT.ptpListLabel}>
  {#each PRODUCTION_STEPS as step (step.area + step.question)}
    <li data-testid="production-step">
      <h2 lang="en">{step.area}: {plain(step.question)}</h2>
      <div class="estimate">
        <Slider
          class="readiness"
          label={TEXT.ptpSlider
            .replace("{area}", step.area)
            .replace("{question}", plain(step.question))}
          value={step.estimate}
          min={0}
          max={100}
          step={5}
          disabled
          aria-valuetext={`${step.estimate}%`}
        />
        <span class="percent">{step.estimate}%</span>
      </div>
      <dl>
        <dt>{TEXT.ptpToday}</dt>
        <dd lang="en">{plain(step.today)}</dd>
        <dt>{TEXT.ptpWhy}</dt>
        <dd lang="en">{step.why}</dd>
      </dl>
    </li>
  {/each}
</ol>

<style>
  .steps {
    padding-inline-start: 1.5rem;
  }
  .steps li {
    margin-block-end: 1.5rem;
  }
  h2 {
    font-size: 1.05rem;
    margin-block-end: 0.25rem;
  }
  .estimate {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    max-inline-size: 32rem;
  }
  .estimate :global(.readiness) {
    flex: 1;
    min-inline-size: 0;
    accent-color: var(--accent);
    opacity: 1;
  }
  .percent {
    font-variant-numeric: tabular-nums;
    min-inline-size: 3.5ch;
    font-weight: 600;
  }
  dl {
    margin-block: 0.5rem 0;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin-inline-start: 0;
    margin-block-end: 0.25rem;
  }
</style>
