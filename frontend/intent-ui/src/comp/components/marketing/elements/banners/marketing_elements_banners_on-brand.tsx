
export function Example() {
  return (
    <div class="flex items-center gap-x-6 bg-indigo-600 px-6 py-2.5 sm:px-3.5 sm:before:flex-1">
      <p class="text-sm/6 text-white">
        <a href="#">
          <strong class="font-semibold">GeneriCon 2023</strong>
          <svg viewBox="0 0 2 2" aria-hidden="true" class="mx-2 inline size-0.5 fill-current">
            <circle r={1} cx={1} cy={1} />
          </svg>
          Join us in Denver from June 7 – 9 to see what’s coming next&nbsp;<span aria-hidden="true">&rarr;</span>
        </a>
      </p>
      <div class="flex flex-1 justify-end">
        <button type="button" class="-m-3 p-3 focus-visible:-outline-offset-4">
          <span class="sr-only">Dismiss</span>
          <XMarkIcon aria-hidden="true" class="size-5 text-white" />
        </button>
      </div>
    </div>
  )
}
