// Simple SolidJS Popover components (Headless UI alternative)
import { JSX, mergeProps } from "solid-js";

export interface PopoverProps {
  class?: string;
  children?: JSX.Element;
}

export function Popover(props: PopoverProps) {
  const merged = mergeProps({ class: '' }, props);
  return <div class={`relative ${merged.class}`}>{props.children}</div>;
}

export interface PopoverButtonProps {
  class?: string;
  children?: JSX.Element;
  onClick?: () => void;
}

export function PopoverButton(props: PopoverButtonProps) {
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

export interface PopoverPanelProps {
  class?: string;
  children?: JSX.Element;
  static?: boolean;
}

export function PopoverPanel(props: PopoverPanelProps) {
  const merged = mergeProps({ class: '' }, props);
  return (
    <div class={`absolute ${merged.class}`}>
      {props.children}
    </div>
  );
}

export interface PopoverGroupProps {
  class?: string;
  children?: JSX.Element;
}

export function PopoverGroup(props: PopoverGroupProps) {
  const merged = mergeProps({ class: '' }, props);
  return <div class={merged.class}>{props.children}</div>;
}
