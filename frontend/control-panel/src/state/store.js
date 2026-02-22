// SPDX-License-Identifier: Apache-2.0
export function createStore(initialValue) {
  let value = initialValue;
  const listeners = new Set();

  return {
    get() {
      return value;
    },
    set(nextOrUpdater) {
      value = typeof nextOrUpdater === 'function' ? nextOrUpdater(value) : nextOrUpdater;
      for (const listener of listeners) listener(value);
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

export function cloneState(value) {
  return JSON.parse(JSON.stringify(value));
}
