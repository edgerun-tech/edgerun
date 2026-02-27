export interface ConstrainedThreeColumnProps {
  class?: string;
}
export function ConstrainedThreeColumn(props: ConstrainedThreeColumnProps): JSX.Element {
  return (
    <div class={props.class || ""}>
<div class="bg-white dark:bg-gray-900 dark:scheme-dark">
<div class="flex min-h-full flex-col">
    <header class="relative shrink-0 bg-gray-900 dark:before:pointer-events-none dark:before:absolute dark:before:inset-0 dark:before:border-b dark:before:border-white/10 dark:before:bg-black/10">
      <div class="relative mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <img src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500" alt="Your Company" class="h-8 w-auto" />
        <div class="flex items-center gap-x-8">
          <button type="button" class="-m-2.5 p-2.5 text-gray-400 hover:text-white">
            <span class="sr-only">View notifications</span>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" data-slot="icon" aria-hidden={true} class="size-6">
              <path d="M14.857 17.082a23.848 23.848 0 0 0 5.454-1.31A8.967 8.967 0 0 1 18 9.75V9A6 6 0 0 0 6 9v.75a8.967 8.967 0 0 1-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 0 1-5.714 0m5.714 0a3 3 0 1 1-5.714 0" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
          <a href="#" class="-m-1.5 p-1.5">
            <span class="sr-only">Your profile</span>
            <img src="https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?ixlib=rb-1.2.1&ixid=eyJhcHBfaWQiOjEyMDd9&auto=format&fit=facearea&facepad=2&w=256&h=256&q=80" alt="" class="size-8 rounded-full bg-gray-800 outline -outline-offset-1 outline-white/10" />
          </a>
        </div>
      </div>
    </header>
<div class="mx-auto w-full max-w-7xl grow lg:flex xl:px-2">
<div class="flex-1 xl:flex">
        <div class="border-b border-gray-200 px-4 py-6 sm:px-6 lg:pl-8 xl:w-64 xl:shrink-0 xl:border-r xl:border-b-0 xl:pl-6 dark:border-white/10">
          <x-placeholder message="Left column area">
            <div class="relative h-[192px] overflow-hidden rounded-xl border border-dashed border-gray-400 opacity-75 xl:h-[608px] dark:border-white/20">
              <svg fill="none" class="absolute inset-0 size-full stroke-gray-900/10 dark:stroke-white/10">
                <defs>
                  <pattern id="pattern-e65c4c0f-2107-4ff8-8f1a-e4204a4fd15f" width="10" height="10" x="0" y="0" patternUnits="userSpaceOnUse">
                    <path d="M-3 13 15-5M-5 5l18-18M-1 21 17 3" />
                  </pattern>
                </defs>
                <rect width="100%" height="100%" fill="url(#pattern-e65c4c0f-2107-4ff8-8f1a-e4204a4fd15f)" stroke="none" />
              </svg>
            </div>
          </x-placeholder>
        </div>
        <div class="px-4 py-6 sm:px-6 lg:pl-8 xl:flex-1 xl:pl-6">
          <x-placeholder message="Main area">
            <div class="relative h-[367px] overflow-hidden rounded-xl border border-dashed border-gray-400 opacity-75 xl:h-[608px] dark:border-white/20">
              <svg fill="none" class="absolute inset-0 size-full stroke-gray-900/10 dark:stroke-white/10">
                <defs>
                  <pattern id="pattern-7b69d9f9-ca30-48c9-a80a-268e7b084e52" width="10" height="10" x="0" y="0" patternUnits="userSpaceOnUse">
                    <path d="M-3 13 15-5M-5 5l18-18M-1 21 17 3" />
                  </pattern>
                </defs>
                <rect width="100%" height="100%" fill="url(#pattern-7b69d9f9-ca30-48c9-a80a-268e7b084e52)" stroke="none" />
              </svg>
            </div>
          </x-placeholder>
        </div>
      </div>
      <div class="shrink-0 border-t border-gray-200 px-4 py-6 sm:px-6 lg:w-96 lg:border-t-0 lg:border-l lg:pr-8 xl:pr-6 dark:border-white/10">
        <x-placeholder message="Right column area">
          <div class="relative h-[256px] overflow-hidden rounded-xl border border-dashed border-gray-400 opacity-75 lg:h-full xl:h-[608px] dark:border-white/20">
            <svg fill="none" class="absolute inset-0 size-full stroke-gray-900/10 dark:stroke-white/10">
              <defs>
                <pattern id="pattern-1b61a508-0497-4b7f-8a04-44300b5c3e3a" width="10" height="10" x="0" y="0" patternUnits="userSpaceOnUse">
                  <path d="M-3 13 15-5M-5 5l18-18M-1 21 17 3" />
                </pattern>
              </defs>
              <rect width="100%" height="100%" fill="url(#pattern-1b61a508-0497-4b7f-8a04-44300b5c3e3a)" stroke="none" />
            </svg>
          </div>
        </x-placeholder>
      </div>
    </div>
  </div>
</div>
    </div>
  );
}
