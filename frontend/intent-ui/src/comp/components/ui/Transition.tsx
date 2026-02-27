// Simple SolidJS Transition component (Headless UI alternative)
import { JSX, mergeProps, Show } from "solid-js";

export interface TransitionProps {
  class?: string;
  children?: JSX.Element;
  show?: boolean;
  enter?: string;
  enterFrom?: string;
  enterTo?: string;
  leave?: string;
  leaveFrom?: string;
  leaveTo?: string;
  appear?: boolean;
}

export function Transition(props: TransitionProps) {
  const merged = mergeProps({ 
    class: '', 
    show: true,
    enter: '',
    enterFrom: '',
    enterTo: '',
    leave: '',
    leaveFrom: '',
    leaveTo: '',
    appear: false
  }, props);
  
  // Simple implementation - just render children
  // Full implementation would handle transitions
  return (
    <Show when={merged.show}>
      <div class={merged.class}>
        {props.children}
      </div>
    </Show>
  );
}

export interface TransitionChildProps {
  class?: string;
  children?: JSX.Element;
}

export function TransitionChild(props: TransitionChildProps) {
  const merged = mergeProps({ class: '' }, props);
  return (
    <div class={merged.class}>
      {props.children}
    </div>
  );
}
