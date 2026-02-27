import { createSignal } from "solid-js";


const navigation = [
  { name: 'Product', href: '#' },
  { name: 'Features', href: '#' },
  { name: 'Marketplace', href: '#' },
  { name: 'Company', href: '#' },
]
export function Example() {
  const [mobileMenuOpen, setMobileMenuOpen] = createSignal(false)
  return (
    <header class="bg-white dark:bg-gray-900">
      <nav aria-label="Global" class="mx-auto flex max-w-7xl items-center justify-between gap-x-6 p-6 lg:px-8">
        <div class="flex lg:flex-1">
          <a href="#" class="-m-1.5 p-1.5">
            <span class="sr-only">Your Company</span>
            <img
              alt=""
              src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
              class="h-8 w-auto dark:hidden"
            />
            <img
              alt=""
              src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500"
              class="h-8 w-auto hidden"
            />
          </a>
        </div>
        <div class="hidden lg:flex lg:gap-x-12">
          {navigation.map((item) => (
            <a href={item.href} class="text-sm/6 font-semibold text-gray-900 dark:text-white">
              {item.name}
            </a>
          ))}
        </div>
        <div class="flex flex-1 items-center justify-end gap-x-6">
          <a href="#" class="hidden text-sm/6 font-semibold text-gray-900 lg:block dark:text-white">
            Log in
          </a>
          <a
            href="#"
            class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500"
          >
            Sign up
          </a>
        </div>
        <div class="flex lg:hidden">
          <button
            type="button"
            onClick={() => setMobileMenuOpen(true)}
            class="-m-2.5 inline-flex items-center justify-center rounded-md p-2.5 text-gray-700 dark:text-gray-400"
          >
            <span class="sr-only">Open main menu</span>
            <Bars3Icon aria-hidden="true" class="size-6" />
          </button>
        </div>
      </nav>
      <Dialog open={mobileMenuOpen} onClose={setMobileMenuOpen} class="lg:hidden">
        <div class="fixed inset-0 z-50" />
        <DialogPanel class="fixed inset-y-0 right-0 z-50 w-full overflow-y-auto bg-white p-6 sm:max-w-sm sm:ring-1 sm:ring-gray-900/10 dark:bg-gray-900 dark:sm:ring-gray-100/10">
          <div class="flex items-center gap-x-6">
            <a href="#" class="-m-1.5 p-1.5">
              <span class="sr-only">Your Company</span>
              <img
                alt=""
                src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                class="h-8 w-auto dark:hidden"
              />
              <img
                alt=""
                src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500"
                class="h-8 w-auto hidden"
              />
            </a>
            <a
              href="#"
              class="ml-auto rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500"
            >
              Sign up
            </a>
            <button
              type="button"
              onClick={() => setMobileMenuOpen(false)}
              class="-m-2.5 rounded-md p-2.5 text-gray-700 dark:text-gray-400"
            >
              <span class="sr-only">Close menu</span>
              <XMarkIcon aria-hidden="true" class="size-6" />
            </button>
          </div>
          <div class="mt-6 flow-root">
            <div class="-my-6 divide-y divide-gray-500/10 dark:divide-white/10">
              <div class="space-y-2 py-6">
                {navigation.map((item) => (
                  <a
                    href={item.href}
                    class="-mx-3 block rounded-lg px-3 py-2 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5"
                  >
                    {item.name}
                  </a>
                ))}
              </div>
              <div class="py-6">
                <a
                  href="#"
                  class="-mx-3 block rounded-lg px-3 py-2.5 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5"
                >
                  Log in
                </a>
              </div>
            </div>
          </div>
        </DialogPanel>
      </Dialog>
    </header>
  )
}
