import { splitProps } from "solid-js";
import { ContextMenu as ContextMenuPrimitive } from "@kobalte/core/context-menu";

import { cx } from "../../registry/lib/cva";

const ContextMenuPortal = ContextMenuPrimitive.Portal;

const ContextMenu = (props) => {
  return <ContextMenuPrimitive data-slot="context-menu" {...props} />;
};

const ContextMenuTrigger = (props) => {
  return <ContextMenuPrimitive.Trigger data-slot="context-menu-trigger" {...props} />;
};

const ContextMenuGroup = (props) => {
  return <ContextMenuPrimitive.Group data-slot="context-menu-group" {...props} />;
};

const ContextMenuSub = (props) => {
  return <ContextMenuPrimitive.Sub data-slot="context-menu-sub" {...props} />;
};

const ContextMenuRadioGroup = (props) => {
  return <ContextMenuPrimitive.RadioGroup data-slot="context-menu-radio-group" {...props} />;
};

const ContextMenuSubTrigger = (props) => {
  const [, rest] = splitProps(props, ["class", "children", "inset"]);

  return (
    <ContextMenuPrimitive.SubTrigger
      data-slot="context-menu-sub-trigger"
      data-inset={props.inset}
      class={cx(
        "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground data-[expanded]:bg-accent data-[expanded]:text-accent-foreground flex cursor-default items-center rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        props.class
      )}
      {...rest}
    >
      {props.children}
      <svg xmlns="http://www.w3.org/2000/svg" class="ml-auto" viewBox="0 0 24 24">
        <path
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="m9 18l6-6l-6-6"
        />
      </svg>
    </ContextMenuPrimitive.SubTrigger>
  );
};

const ContextMenuSubContent = (props) => {
  const [, rest] = splitProps(props, ["class"]);

  return (
    <ContextMenuPrimitive.SubContent
      data-slot="context-menu-sub-content"
      class={cx(
        "bg-popover text-popover-foreground data-[expanded]:animate-in data-[closed]:animate-out data-[closed]:fade-out-0 data-[expanded]:fade-in-0 data-[closed]:zoom-out-95 data-[expanded]:zoom-in-95 z-50 min-w-[8rem] origin-(--kb-menu-content-transform-origin) overflow-hidden rounded-md border p-1 shadow-lg outline-none",
        "[[data-popper-positioner][style*='--kb-popper-content-transform-origin:_top']>[data-slot=context-menu-sub-content]]:slide-in-from-top-2 [[data-popper-positioner][style*='--kb-popper-content-transform-origin:_bottom']>[data-slot=context-menu-sub-content]]:slide-in-from-bottom-2 [[data-popper-positioner][style*='--kb-popper-content-transform-origin:_left']>[data-slot=context-menu-sub-content]]:slide-in-from-left-2 [[data-popper-positioner][style*='--kb-popper-content-transform-origin:_right']>[data-slot=context-menu-sub-content]]:slide-in-from-right-2",
        props.class
      )}
      {...rest}
    />
  );
};

const ContextMenuContent = (props) => {
  const [, rest] = splitProps(props, ["class"]);

  return (
    <ContextMenuPrimitive.Content
      data-slot="context-menu-content"
      class={cx(
        "bg-popover text-popover-foreground data-[expanded]:animate-in data-[closed]:animate-out data-[closed]:fade-out-0 data-[expanded]:fade-in-0 data-[closed]:zoom-out-95 data-[expanded]:zoom-in-95 z-50 min-w-[8rem] origin-(--kb-menu-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-md border p-1 shadow-md outline-none",
        "[[data-popper-positioner][style*='--kb-popper-content-transform-origin:_top']>[data-slot=context-menu-content]]:slide-in-from-top-2 [[data-popper-positioner][style*='--kb-popper-content-transform-origin:_bottom']>[data-slot=context-menu-content]]:slide-in-from-bottom-2 [[data-popper-positioner][style*='--kb-popper-content-transform-origin:_left']>[data-slot=context-menu-content]]:slide-in-from-left-2 [[data-popper-positioner][style*='--kb-popper-content-transform-origin:_right']>[data-slot=context-menu-content]]:slide-in-from-right-2",
        props.class
      )}
      {...rest}
    />
  );
};

const ContextMenuItem = (props) => {
  const [, rest] = splitProps(props, ["class", "inset", "variant"]);

  return (
    <ContextMenuPrimitive.Item
      data-slot="context-menu-item"
      data-inset={props.inset}
      data-variant={props.variant}
      class={cx(
        "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground data-[variant=destructive]:text-destructive data-[variant=destructive]:data-[highlighted]:bg-destructive/10 dark:data-[variant=destructive]:data-[highlighted]:bg-destructive/20 data-[variant=destructive]:data-[highlighted]:text-destructive data-[variant=destructive]:*:[svg]:!text-destructive [&_svg:not([class*='text-'])]:text-muted-foreground relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        props.class
      )}
      {...rest}
    />
  );
};

const ContextMenuCheckboxItem = (props) => {
  const [, rest] = splitProps(props, ["class", "children"]);

  return (
    <ContextMenuPrimitive.CheckboxItem
      data-slot="context-menu-checkbox-item"
      class={cx(
        "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        props.class
      )}
      {...rest}
    >
      <span class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center">
        <ContextMenuPrimitive.ItemIndicator as="svg" xmlns="http://www.w3.org/2000/svg" class="size-4" viewBox="0 0 24 24">
          <path
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M20 6L9 17l-5-5"
          />
        </ContextMenuPrimitive.ItemIndicator>
      </span>
      {props.children}
    </ContextMenuPrimitive.CheckboxItem>
  );
};

const ContextMenuRadioItem = (props) => {
  const [, rest] = splitProps(props, ["class", "children"]);

  return (
    <ContextMenuPrimitive.RadioItem
      data-slot="context-menu-radio-item"
      class={cx(
        "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground relative flex cursor-default items-center gap-2 rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        props.class
      )}
      {...rest}
    >
      <span class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center">
        <ContextMenuPrimitive.ItemIndicator as="svg" xmlns="http://www.w3.org/2000/svg" class="size-2" viewBox="0 0 24 24">
          <circle
            cx="12"
            cy="12"
            r="10"
            fill="currentColor"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
          />
        </ContextMenuPrimitive.ItemIndicator>
      </span>
      {props.children}
    </ContextMenuPrimitive.RadioItem>
  );
};

const ContextMenuGroupLabel = (props) => {
  const [, rest] = splitProps(props, ["class", "inset"]);

  return (
    <ContextMenuPrimitive.GroupLabel
      as="div"
      data-slot="context-menu-group-label"
      data-inset={props.inset}
      class={cx("text-foreground my-1.5 px-2 text-sm font-medium data-[inset]:pl-8", props.class)}
      {...rest}
    />
  );
};

const ContextMenuItemLabel = (props) => {
  const [, rest] = splitProps(props, ["class", "inset"]);

  return (
    <ContextMenuPrimitive.ItemLabel
      data-slot="context-menu-item-label"
      data-inset={props.inset}
      class={cx("text-foreground px-2 py-1.5 text-sm font-medium data-[inset]:pl-8", props.class)}
      {...rest}
    />
  );
};

const ContextMenuSeparator = (props) => {
  const [, rest] = splitProps(props, ["class"]);

  return <ContextMenuPrimitive.Separator data-slot="context-menu-separator" class={cx("bg-border -mx-1 my-1 h-px", props.class)} {...rest} />;
};

const ContextMenuShortcut = (props) => {
  const [, rest] = splitProps(props, ["class"]);

  return <span data-slot="context-menu-shortcut" class={cx("text-muted-foreground ml-auto text-xs tracking-widest", props.class)} {...rest} />;
};

export {
  ContextMenu,
  ContextMenuCheckboxItem,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuItemLabel,
  ContextMenuPortal,
  ContextMenuRadioGroup,
  ContextMenuRadioItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger
};
