import { splitProps } from "solid-js";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

function cn(...classes) {
  return twMerge(clsx(classes));
}

function Kbd(props) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <kbd
      class={cn(
        "inline-flex min-h-6 min-w-6 items-center justify-center rounded-md border border-neutral-700 bg-neutral-900 px-1.5 py-0.5 font-mono text-[11px] font-semibold leading-none text-neutral-300 shadow-sm",
        local.class
      )}
      {...rest}
    >
      {local.children}
    </kbd>
  );
}

function KbdGroup(props) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <span class={cn("inline-flex items-center gap-1.5 align-middle", local.class)} {...rest}>
      {local.children}
    </span>
  );
}

export { Kbd, KbdGroup };
