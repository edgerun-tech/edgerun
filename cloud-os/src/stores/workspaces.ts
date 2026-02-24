import { createSignal } from 'solid-js';

const [activeWorkspaceId, setActiveWorkspaceId] = createSignal('default');

export { setActiveWorkspaceId };

export function getActiveWorkspaceId(): string {
  return activeWorkspaceId();
}
