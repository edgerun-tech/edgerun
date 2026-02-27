import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cx(...inputs) {
  return twMerge(clsx(inputs));
}
