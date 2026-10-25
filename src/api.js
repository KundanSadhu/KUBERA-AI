/*
 * Development adapter.
 *
 * The browser prototype stores state in localStorage. The production implementation
 * replaces these operations with narrow Tauri invoke wrappers backed by SQLite.
 */
export function createPrototypeApi(store) {
  return {
    getState: () => store.getState(),
    update: (mutator) => store.update(mutator),
    reset: () => store.reset(),
    async bootstrap() {
      return { engine: store.getState().engine, mode: 'prototype' };
    }
  };
}
