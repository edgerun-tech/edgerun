// Simple SolidJS Disclosure components (Headless UI alternative)
import { JSX, mergeProps } from "solid-js";

export interface DisclosureProps {
  class?: string;
  children?: JSX.Element;
  defaultOpen?: boolean;
}

export function Disclosure(props: DisclosureProps) {
  const merged = mergeProps({ class: '', defaultOpen: false }, props);
  
  return (
    <div class={`relative ${merged.class}`}>
      {props.children}
    </div>
  );
}

export interface DisclosureButtonProps {
  class?: string;
  children?: JSX.Element;
  onClick?: () => void;
}

export function DisclosureButton(props: DisclosureButtonProps) {
  const merged = mergeProps({ class: '' }, props);
  return (
    <button 
      type="button"
      class={merged.class}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
}

export interface DisclosurePanelProps {
  class?: string;
  children?: JSX.Element;
  static?: boolean;
}

export function DisclosurePanel(props: DisclosurePanelProps) {
  const merged = mergeProps({ class: '', static: false }, props);
  
  // For now, just render - proper implementation would use context
  return (
    <div class={merged.class}>
      {props.children}
    </div>
  );
}
