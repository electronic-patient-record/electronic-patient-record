// jsdom lays nothing out and has no ResizeObserver, which Svelte's `bind:offsetHeight` needs (`PatientBanner.svelte`).
// One that never calls back: in a unit test nothing has a size.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
