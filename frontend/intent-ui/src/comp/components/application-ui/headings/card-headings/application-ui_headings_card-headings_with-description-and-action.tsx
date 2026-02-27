export interface WithDescriptionAndActionProps {
  class?: string;
}
export function WithDescriptionAndAction(props: WithDescriptionAndActionProps): JSX.Element {
  return (
    <div class={props.class || ""}>
<div class="bg-gray-100 dark:bg-gray-900 dark:scheme-dark">
  <div class="mx-auto max-w-7xl py-6 sm:px-6 lg:px-8">
    <div class="mx-auto max-w-none">
      <div class="overflow-hidden bg-white sm:rounded-lg sm:shadow-sm dark:bg-gray-800/50 dark:shadow-none dark:outline dark:-outline-offset-1 dark:outline-white/10">
        <div class="border-b border-gray-200 px-4 py-5 sm:px-6 dark:border-white/10">
          <div class="-mt-4 -ml-4 flex flex-wrap items-center justify-between sm:flex-nowrap">
            <div class="mt-4 ml-4">
              <h3 class="text-base font-semibold text-gray-900 dark:text-white">Job Postings</h3>
              <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">Lorem ipsum dolor sit amet consectetur adipisicing elit quam corrupti consectetur.</p>
            </div>
            <div class="mt-4 ml-4 shrink-0">
              <button type="button" class="relative inline-flex items-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:shadow-none dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500">Create new job</button>
            </div>
          </div>
        </div>
        <ul role="list" class="divide-y divide-gray-200 opacity-25 dark:divide-white/10">
          <li>
            <a href="#" class="block hover:bg-gray-50 dark:hover:bg-white/5">
              <div class="px-4 py-4 sm:px-6">
                <div class="flex items-center justify-between">
                  <div class="truncate text-sm font-medium text-indigo-600 dark:text-indigo-400">Back End Developer</div>
                  <div class="ml-2 flex shrink-0">
                    <span class="inline-flex items-center rounded-full bg-green-50 px-2 py-1 text-xs font-medium text-green-700 inset-ring inset-ring-green-600/20 dark:bg-green-900/30 dark:text-green-400 dark:inset-ring-green-500/30">Full-time</span>
                  </div>
                </div>
                <div class="mt-2 flex justify-between">
                  <div class="sm:flex">
                    <div class="flex items-center text-sm text-gray-500 dark:text-gray-400">
                      <svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden={true} class="mr-1.5 size-5 shrink-0 text-gray-400 dark:text-gray-500">
                        <path d="M7 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM14.5 9a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM1.615 16.428a1.224 1.224 0 0 1-.569-1.175 6.002 6.002 0 0 1 11.908 0c.058.467-.172.92-.57 1.174A9.953 9.953 0 0 1 7 18a9.953 9.953 0 0 1-5.385-1.572ZM14.5 16h-.106c.07-.297.088-.611.048-.933a7.47 7.47 0 0 0-1.588-3.755 4.502 4.502 0 0 1 5.874 2.636.818.818 0 0 1-.36.98A7.465 7.465 0 0 1 14.5 16Z" />
                      </svg>
                      Engineering
                    </div>
                  </div>
                  <div class="ml-2 flex items-center text-sm text-gray-500 dark:text-gray-400">
                    <svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden={true} class="mr-1.5 size-5 shrink-0 text-gray-400 dark:text-gray-500">
                      <path d="m9.69 18.933.003.001C9.89 19.02 10 19 10 19s.11.02.308-.066l.002-.001.006-.003.018-.008a5.741 5.741 0 0 0 .281-.14c.186-.096.446-.24.757-.433.62-.384 1.445-.966 2.274-1.765C15.302 14.988 17 12.493 17 9A7 7 0 1 0 3 9c0 3.492 1.698 5.988 3.355 7.584a13.731 13.731 0 0 0 2.273 1.765 11.842 11.842 0 0 0 .976.544l.062.029.018.008.006.003ZM10 11.25a2.25 2.25 0 1 0 0-4.5 2.25 2.25 0 0 0 0 4.5Z" clip-rule="evenodd" fill-rule="evenodd" />
                    </svg>
                    Remote
                  </div>
                </div>
              </div>
            </a>
          </li>
          <li>
            <a href="#" class="block hover:bg-gray-50 dark:hover:bg-white/5">
              <div class="px-4 py-4 sm:px-6">
                <div class="flex items-center justify-between">
                  <div class="truncate text-sm font-medium text-indigo-600 dark:text-indigo-400">Front End Developer</div>
                  <div class="ml-2 flex shrink-0">
                    <span class="inline-flex items-center rounded-full bg-green-50 px-2 py-1 text-xs font-medium text-green-700 inset-ring inset-ring-green-600/20 dark:bg-green-900/30 dark:text-green-400 dark:inset-ring-green-500/30">Full-time</span>
                  </div>
                </div>
                <div class="mt-2 flex justify-between">
                  <div class="sm:flex">
                    <div class="flex items-center text-sm text-gray-500 dark:text-gray-400">
                      <svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden={true} class="mr-1.5 size-5 shrink-0 text-gray-400 dark:text-gray-500">
                        <path d="M7 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM14.5 9a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM1.615 16.428a1.224 1.224 0 0 1-.569-1.175 6.002 6.002 0 0 1 11.908 0c.058.467-.172.92-.57 1.174A9.953 9.953 0 0 1 7 18a9.953 9.953 0 0 1-5.385-1.572ZM14.5 16h-.106c.07-.297.088-.611.048-.933a7.47 7.47 0 0 0-1.588-3.755 4.502 4.502 0 0 1 5.874 2.636.818.818 0 0 1-.36.98A7.465 7.465 0 0 1 14.5 16Z" />
                      </svg>
                      Engineering
                    </div>
                  </div>
                  <div class="ml-2 flex items-center text-sm text-gray-500 dark:text-gray-400">
                    <svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden={true} class="mr-1.5 size-5 shrink-0 text-gray-400 dark:text-gray-500">
                      <path d="m9.69 18.933.003.001C9.89 19.02 10 19 10 19s.11.02.308-.066l.002-.001.006-.003.018-.008a5.741 5.741 0 0 0 .281-.14c.186-.096.446-.24.757-.433.62-.384 1.445-.966 2.274-1.765C15.302 14.988 17 12.493 17 9A7 7 0 1 0 3 9c0 3.492 1.698 5.988 3.355 7.584a13.731 13.731 0 0 0 2.273 1.765 11.842 11.842 0 0 0 .976.544l.062.029.018.008.006.003ZM10 11.25a2.25 2.25 0 1 0 0-4.5 2.25 2.25 0 0 0 0 4.5Z" clip-rule="evenodd" fill-rule="evenodd" />
                    </svg>
                    Remote
                  </div>
                </div>
              </div>
            </a>
          </li>
          <li>
            <a href="#" class="block hover:bg-gray-50 dark:hover:bg-white/5">
              <div class="px-4 py-4 sm:px-6">
                <div class="flex items-center justify-between">
                  <div class="truncate text-sm font-medium text-indigo-600 dark:text-indigo-400">User Interface Designer</div>
                  <div class="ml-2 flex shrink-0">
                    <span class="inline-flex items-center rounded-full bg-green-50 px-2 py-1 text-xs font-medium text-green-700 inset-ring inset-ring-green-600/20 dark:bg-green-900/30 dark:text-green-400 dark:inset-ring-green-500/30">Full-time</span>
                  </div>
                </div>
                <div class="mt-2 flex justify-between">
                  <div class="sm:flex">
                    <div class="flex items-center text-sm text-gray-500 dark:text-gray-400">
                      <svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden={true} class="mr-1.5 size-5 shrink-0 text-gray-400 dark:text-gray-500">
                        <path d="M7 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM14.5 9a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM1.615 16.428a1.224 1.224 0 0 1-.569-1.175 6.002 6.002 0 0 1 11.908 0c.058.467-.172.92-.57 1.174A9.953 9.953 0 0 1 7 18a9.953 9.953 0 0 1-5.385-1.572ZM14.5 16h-.106c.07-.297.088-.611.048-.933a7.47 7.47 0 0 0-1.588-3.755 4.502 4.502 0 0 1 5.874 2.636.818.818 0 0 1-.36.98A7.465 7.465 0 0 1 14.5 16Z" />
                      </svg>
                      Design
                    </div>
                  </div>
                  <div class="ml-2 flex items-center text-sm text-gray-500 dark:text-gray-400">
                    <svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden={true} class="mr-1.5 size-5 shrink-0 text-gray-400 dark:text-gray-500">
                      <path d="m9.69 18.933.003.001C9.89 19.02 10 19 10 19s.11.02.308-.066l.002-.001.006-.003.018-.008a5.741 5.741 0 0 0 .281-.14c.186-.096.446-.24.757-.433.62-.384 1.445-.966 2.274-1.765C15.302 14.988 17 12.493 17 9A7 7 0 1 0 3 9c0 3.492 1.698 5.988 3.355 7.584a13.731 13.731 0 0 0 2.273 1.765 11.842 11.842 0 0 0 .976.544l.062.029.018.008.006.003ZM10 11.25a2.25 2.25 0 1 0 0-4.5 2.25 2.25 0 0 0 0 4.5Z" clip-rule="evenodd" fill-rule="evenodd" />
                    </svg>
                    Remote
                  </div>
                </div>
              </div>
            </a>
          </li>
        </ul>
      </div>
    </div>
  </div>
</div>
    </div>
  );
}
