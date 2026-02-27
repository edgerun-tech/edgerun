
export function Example() {
  return (
    <div class="relative flex items-center justify-between gap-x-6 bg-gray-900 px-6 py-2.5 sm:pr-3.5 lg:pl-8 dark:bg-gray-800 dark:after:pointer-events-none dark:after:absolute dark:after:inset-x-0 dark:after:bottom-0 dark:after:h-px dark:after:bg-white/10">
      <p class="text-sm/6 text-white">
        <a href="#">
          <strong class="font-semibold">GeneriCon 2023</strong>
          <svg viewBox="0 0 2 2" aria-hidden="true" class="mx-2 inline size-0.5 fill-current">
            <circle r={1} cx={1} cy={1} />
          </svg>
          Join us in Denver from June 7 – 9 to see what’s coming next&nbsp;<span aria-hidden="true">&rarr;</span>
        </a>
      </p>
      <button type="button" class="-m-3 flex-none p-3 focus-visible:-outline-offset-4">
        <span class="sr-only">Dismiss</span>
        <XMarkIcon aria-hidden="true" class="size-5 text-white" />
      </button>
    </div>
  )
}
