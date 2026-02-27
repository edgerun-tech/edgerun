import { createSignal } from "solid-js";

import {
  ArrowPathIcon,
  Bars3Icon,
  ChartPieIcon,
  CursorArrowRaysIcon,
  FingerPrintIcon,
  SquaresPlusIcon,
  XMarkIcon,
} from "../../../ui/compat-runtime"

const products = [
  { name: 'Analytics', description: 'Get a better understanding of your traffic', href: '#', icon: ChartPieIcon },
  { name: 'Engagement', description: 'Speak directly to your customers', href: '#', icon: CursorArrowRaysIcon },
  { name: 'Security', description: 'Your customers’ data will be safe and secure', href: '#', icon: FingerPrintIcon },
  { name: 'Integrations', description: 'Connect with third-party tools', href: '#', icon: SquaresPlusIcon },
  { name: 'Automations', description: 'Build strategic funnels that will convert', href: '#', icon: ArrowPathIcon },
]
const callsToAction = [
  { name: 'Watch demo', href: '#', icon: PlayCircleIcon },
  { name: 'Contact sales', href: '#', icon: PhoneIcon },
]
const company = [
  { name: 'About us', href: '#', description: 'Learn more about our company values and mission to empower others' },
  { name: 'Careers', href: '#', description: 'Looking for you next career opportunity? See all of our open positions' },
  {
    name: 'Support',
    href: '#',
    description: 'Get in touch with our dedicated support team or reach out on our community forums',
  },
  { name: 'Blog', href: '#', description: 'Read our latest announcements and get perspectives from our team' },
]
export function Example() {
  const [mobileMenuOpen, setMobileMenuOpen] = createSignal(false)
  return (
    <header class="bg-white dark:bg-gray-900">
      <nav aria-label="Global" class="mx-auto flex max-w-7xl items-center justify-between p-6 lg:px-8">
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
        <PopoverGroup class="hidden lg:flex lg:gap-x-12">
          <Popover class="relative">
            <PopoverButton class="flex items-center gap-x-1 text-sm/6 font-semibold text-gray-900 dark:text-white">
              Product
              <ChevronDownIcon aria-hidden="true" class="size-5 flex-none text-gray-400 dark:text-gray-500" />
            </PopoverButton>
            <PopoverPanel
              transition
              class="absolute left-1/2 z-10 mt-3 w-screen max-w-md -translate-x-1/2 overflow-hidden rounded-3xl bg-white shadow-lg outline-1 outline-gray-900/5 transition data-closed:translate-y-1 data-closed:opacity-0 data-enter:duration-200 data-enter:ease-out data-leave:duration-150 data-leave:ease-in dark:bg-gray-800 dark:shadow-none dark:-outline-offset-1 dark:outline-white/10"
            >
              <div class="p-4">
                {products.map((item) => (
                  <div
                    class="group relative flex items-center gap-x-6 rounded-lg p-4 text-sm/6 hover:bg-gray-50 dark:hover:bg-white/5"
                  >
                    <div class="flex size-11 flex-none items-center justify-center rounded-lg bg-gray-50 group-hover:bg-white dark:bg-gray-700/50 dark:group-hover:bg-gray-700">
                      <item.icon
                        aria-hidden="true"
                        class="size-6 text-gray-600 group-hover:text-indigo-600 dark:text-gray-400 dark:group-hover:text-white"
                      />
                    </div>
                    <div class="flex-auto">
                      <a href={item.href} class="block font-semibold text-gray-900 dark:text-white">
                        {item.name}
                        <span class="absolute inset-0" />
                      </a>
                      <p class="mt-1 text-gray-600 dark:text-gray-400">{item.description}</p>
                    </div>
                  </div>
                ))}
              </div>
              <div class="grid grid-cols-2 divide-x divide-gray-900/5 bg-gray-50 dark:divide-white/10 dark:bg-gray-700/50">
                {callsToAction.map((item) => (
                  <a
                    href={item.href}
                    class="flex items-center justify-center gap-x-2.5 p-3 text-sm/6 font-semibold text-gray-900 hover:bg-gray-100 dark:text-white dark:hover:bg-gray-700/50"
                  >
                    <item.icon aria-hidden="true" class="size-5 flex-none text-gray-400 dark:text-gray-500" />
                    {item.name}
                  </a>
                ))}
              </div>
            </PopoverPanel>
          </Popover>
          <a href="#" class="text-sm/6 font-semibold text-gray-900 dark:text-white">
            Features
          </a>
          <a href="#" class="text-sm/6 font-semibold text-gray-900 dark:text-white">
            Marketplace
          </a>
          <Popover class="relative">
            <PopoverButton class="flex items-center gap-x-1 text-sm/6 font-semibold text-gray-900 dark:text-white">
              Company
              <ChevronDownIcon aria-hidden="true" class="size-5 flex-none text-gray-400 dark:text-gray-500" />
            </PopoverButton>
            <PopoverPanel
              transition
              class="absolute left-1/2 z-10 mt-3 w-96 -translate-x-1/2 rounded-3xl bg-white p-4 shadow-lg outline-1 outline-gray-900/5 transition data-closed:translate-y-1 data-closed:opacity-0 data-enter:duration-200 data-enter:ease-out data-leave:duration-150 data-leave:ease-in dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10"
            >
              {company.map((item) => (
                <div class="relative rounded-lg p-4 hover:bg-gray-50 dark:hover:bg-white/5">
                  <a href={item.href} class="block text-sm/6 font-semibold text-gray-900 dark:text-white">
                    {item.name}
                    <span class="absolute inset-0" />
                  </a>
                  <p class="mt-1 text-sm/6 text-gray-600 dark:text-gray-400">{item.description}</p>
                </div>
              ))}
            </PopoverPanel>
          </Popover>
        </PopoverGroup>
        <div class="hidden lg:flex lg:flex-1 lg:justify-end">
          <a href="#" class="text-sm/6 font-semibold text-gray-900 dark:text-white">
            Log in <span aria-hidden="true">&rarr;</span>
          </a>
        </div>
      </nav>
      <Dialog open={mobileMenuOpen} onClose={setMobileMenuOpen} class="lg:hidden">
        <div class="fixed inset-0 z-10" />
        <DialogPanel class="fixed inset-y-0 right-0 z-10 flex w-full flex-col justify-between overflow-y-auto bg-white sm:max-w-sm sm:ring-1 sm:ring-gray-900/10 dark:bg-gray-900 dark:sm:ring-white/10">
          <div class="p-6">
            <div class="flex items-center justify-between">
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
                  {products.map((item) => (
                    <a
                      href={item.href}
                      class="group -mx-3 flex items-center gap-x-6 rounded-lg p-3 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5"
                    >
                      <div class="flex size-11 flex-none items-center justify-center rounded-lg bg-gray-50 group-hover:bg-white dark:bg-gray-800 dark:group-hover:bg-gray-700">
                        <item.icon
                          aria-hidden="true"
                          class="size-6 text-gray-600 group-hover:text-indigo-600 dark:text-gray-300 dark:group-hover:text-white"
                        />
                      </div>
                      {item.name}
                    </a>
                  ))}
                </div>
                <div class="space-y-2 py-6">
                  <a
                    href="#"
                    class="-mx-3 block rounded-lg px-3 py-2 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5"
                  >
                    Features
                  </a>
                  <a
                    href="#"
                    class="-mx-3 block rounded-lg px-3 py-2 text-base/7 font-semibold text-gray-900 hover:bg-gray-50 dark:text-white dark:hover:bg-white/5"
                  >
                    Marketplace
                  </a>
                  {company.map((item) => (
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
          </div>
          <div class="sticky bottom-0 grid grid-cols-2 divide-x divide-gray-900/5 bg-gray-50 text-center dark:divide-white/5 dark:bg-gray-800/50">
            {callsToAction.map((item) => (
              <a
                href={item.href}
                class="p-3 text-base/7 font-semibold text-gray-900 hover:bg-gray-100 dark:text-white dark:hover:bg-gray-800"
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
