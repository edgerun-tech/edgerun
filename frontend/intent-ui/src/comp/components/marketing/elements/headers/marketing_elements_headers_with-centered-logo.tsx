import { createSignal } from "solid-js";


const navigation = [
  { name: 'Product', href: '#' },
  { name: 'Features', href: '#' },
  { name: 'Company', href: '#' },
]
export function Example() {
  const [mobileMenuOpen, setMobileMenuOpen] = createSignal(false)
  return (
    <header class="bg-white dark:bg-gray-900">
      <nav aria-label="Global" class="mx-auto flex max-w-7xl items-center justify-between p-6 lg:px-8">
        <div class="flex flex-1">
          <div class="hidden lg:flex lg:gap-x-12">
            {navigation.map((item) => (
              <a href={item.href} class="text-sm/6 font-semibold text-gray-900 dark:text-white">
                {item.name}
              </a>
            ))}
          </div>
          <div class="flex lg:hidden">
            <button
              type="button"
              onClick={() => setMobileMenuOpen(true)}
              class="-m-2.5 inline-flex items-center justify-center rounded-md p-2.5 text-gray-700 dark:text-gray-400 dark:hover:text-white"
            >
              <span class="sr-only">Open main menu</span>
              <Bars3Icon aria-hidden="true" class="size-6" />
            </button>
          </div>
        </div>
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
        <div class="flex flex-1 justify-end">
          <a href="#" class="text-sm/6 font-semibold text-gray-900 dark:text-white">
            Log in <span aria-hidden="true">&rarr;</span>
          </a>
        </div>
      </nav>
      <Dialog open={mobileMenuOpen} onClose={setMobileMenuOpen} class="lg:hidden">
        <div class="fixed inset-0 z-10" />
        <DialogPanel class="fixed inset-y-0 left-0 z-10 w-full overflow-y-auto bg-white px-6 py-6 dark:bg-gray-900">
          <div class="flex items-center justify-between">
            <div class="flex flex-1">
              <button
                type="button"
                onClick={() => setMobileMenuOpen(false)}
                class="-m-2.5 rounded-md p-2.5 text-gray-700 dark:text-gray-400 dark:hover:text-white"
              >
                <span class="sr-only">Close menu</span>
                <XMarkIcon aria-hidden="true" class="size-6" />
              </button>
            </div>
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
            <div class="flex flex-1 justify-end">
              <a href="#" class="text-sm/6 font-semibold text-gray-900 dark:text-white">
                Log in <span aria-hidden="true">&rarr;</span>
              </a>
            </div>
          </div>
          <div class="mt-6 space-y-2">
            {navigation.map((item) => (
              <a
                href={item.href}
                class="-mx-3 block rounded-lg px-3 py-2 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5"
              >
                {item.name}
              </a>
            ))}
          </div>
        </DialogPanel>
      </Dialog>
    </header>
  )
}
