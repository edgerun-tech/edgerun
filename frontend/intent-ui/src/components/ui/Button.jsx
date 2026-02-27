import { Show } from "solid-js";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function Button(props) {
  const {
    variant = "secondary",
    size = "md",
    isLoading = false,
    leftIcon,
    rightIcon,
    children,
    class: className,
    disabled,
    ...rest
  } = props;
  const baseStyles = "inline-flex items-center justify-center font-medium transition-all cursor-pointer focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-neutral-800 disabled:opacity-50 disabled:cursor-not-allowed";
  const variantStyles = {
    primary: "bg-blue-600 text-white hover:bg-blue-500 focus:ring-blue-500 rounded-lg",
    secondary: "bg-neutral-700 text-neutral-300 hover:bg-neutral-600 focus:ring-neutral-500 rounded-lg",
    danger: "bg-red-600 text-white hover:bg-red-500 focus:ring-red-500 rounded-lg",
    ghost: "bg-transparent text-neutral-400 hover:text-white hover:bg-neutral-700 focus:ring-neutral-500 rounded-lg",
    icon: "bg-transparent text-neutral-400 hover:text-white hover:bg-neutral-700 focus:ring-blue-500 rounded"
  };
  const sizeStyles = {
    sm: "px-2.5 py-1.5 text-xs gap-1.5",
    md: "px-3 py-1.5 text-sm gap-2",
    lg: "px-4 py-2 text-base gap-2"
  };
  const iconSizeStyles = {
    sm: "p-1.5",
    md: "p-2",
    lg: "p-2.5"
  };
  const isIconOnly = variant === "icon" || !children && (leftIcon || rightIcon);
  return <button
    type="button"
    disabled={disabled || isLoading}
    class={cn(
      baseStyles,
      isIconOnly ? iconSizeStyles[size] : sizeStyles[size],
      variantStyles[variant],
      className
    )}
    {...rest}
  >
      <Show when={isLoading}>
        <svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
        </svg>
      </Show>
      <Show when={!isLoading && leftIcon}>{leftIcon}</Show>
      <Show when={!isLoading && children}>{children}</Show>
      <Show when={!isLoading && rightIcon}>{rightIcon}</Show>
    </button>;
}
export {
  Button
};
