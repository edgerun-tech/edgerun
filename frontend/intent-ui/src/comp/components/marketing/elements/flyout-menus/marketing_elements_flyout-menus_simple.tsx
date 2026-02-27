

const solutions = [
  { name: 'Analytics', href: '#' },
  { name: 'Engagement', href: '#' },
  { name: 'Security', href: '#' },
  { name: 'Integrations', href: '#' },
  { name: 'Automations', href: '#' },
  { name: 'Reports', href: '#' },
]
export function Example() {
  return (
    <Popover class="relative">
      <PopoverButton class="inline-flex items-center gap-x-1 text-sm/6 font-semibold text-gray-900 dark:text-white">
        <span>Solutions</span>
        <ChevronDownIcon aria-hidden="true" class="size-5" />
      </PopoverButton>
      <PopoverPanel
        transition
        class="absolute left-1/2 z-10 mt-5 flex w-screen max-w-min -translate-x-1/2 bg-transparent px-4 transition data-closed:translate-y-1 data-closed:opacity-0 data-enter:duration-200 data-enter:ease-out data-leave:duration-150 data-leave:ease-in"
      >
        <div class="w-56 shrink rounded-xl bg-white p-4 text-sm/6 font-semibold text-gray-900 shadow-lg outline-1 outline-gray-900/5 dark:bg-gray-800 dark:text-white dark:shadow-none dark:outline dark:-outline-offset-1 dark:outline-white/10">
          {solutions.map((item) => (
            <a href={item.href} class="block p-2 hover:text-indigo-600 dark:hover:text-indigo-400">
              {item.name}
            </a>
          ))}
        </div>
      </PopoverPanel>
    </Popover>
  )
}
