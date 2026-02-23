import { createSignal, createRoot } from 'solid-js';

export interface KeyBinding {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
  description: string;
  category: string;
  action: () => void;
}

const [bindings, setBindings] = createSignal<KeyBinding[]>([]);
const [helpVisible, setHelpVisible] = createSignal(false);
const [initialized, setInitialized] = createSignal(false);

export function registerBinding(binding: KeyBinding) {
  setBindings(prev => prev.filter(b => 
    !(b.key === binding.key && 
      !!b.ctrl === !!binding.ctrl && 
      !!b.meta === !!binding.meta && 
      !!b.shift === !!binding.shift && 
      !!b.alt === !!binding.alt)
  ));
  setBindings(prev => [...prev, binding]);
}

export function unregisterBinding(key: string) {
  setBindings(prev => prev.filter(b => b.key !== key));
}

export function getBindings() {
  return bindings();
}

export function toggleHelp() {
  setHelpVisible(prev => !prev);
}

export function isHelpVisible() {
  return helpVisible();
}

export function showKeybindingsHelp() {
  setHelpVisible(true);
}

export function hideKeybindingsHelp() {
  setHelpVisible(false);
}

export function formatKey(binding: KeyBinding): string {
  const parts: string[] = [];
  if (binding.ctrl) parts.push('Ctrl');
  if (binding.meta) parts.push('⌘');
  if (binding.shift) parts.push('⇧');
  if (binding.alt) parts.push('⌥');
  parts.push(binding.key.toUpperCase());
  return parts.join('+');
}

export function initKeybindings() {
  if (initialized() || typeof window === 'undefined') return;
  setInitialized(true);
  
  // Import and register defaults
  // Note: IntentBar is always visible now
  
  registerBinding({
    key: '?',
    shift: true,
    description: 'Show keyboard shortcuts',
    category: 'Help',
    action: () => toggleHelp()
  });

  // Global keyboard listener
  const handleKeyDown = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;
    
    const isCtrlOrMeta = e.ctrlKey || e.metaKey;
    const isIntentBarShortcut = isCtrlOrMeta && e.key.toLowerCase() === ' ';
    
    if (isInput && !isIntentBarShortcut) {
      return;
    }

    const allBindings = bindings();
    
    for (const binding of allBindings) {
      const keyMatch = e.key.toLowerCase() === binding.key.toLowerCase();
      const ctrlMatch = !!binding.ctrl === (e.ctrlKey || e.metaKey);
      const metaMatch = !!binding.meta === e.metaKey;
      const shiftMatch = !!binding.shift === e.shiftKey;
      const altMatch = !!binding.alt === e.altKey;
      
      if (!binding.ctrl && !binding.meta && !binding.shift && !binding.alt) {
        if (keyMatch) {
          e.preventDefault();
          binding.action();
          return;
        }
      } else {
        if (keyMatch && ctrlMatch && metaMatch && shiftMatch && altMatch) {
          e.preventDefault();
          binding.action();
          return;
        }
      }
    }
  };
  
  document.addEventListener('keydown', handleKeyDown);
}
