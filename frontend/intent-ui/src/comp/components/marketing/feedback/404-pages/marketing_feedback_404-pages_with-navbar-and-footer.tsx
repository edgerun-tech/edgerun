import { createSignal } from "solid-js";


const navigation = [
  { name: 'Product', href: '#' },
  { name: 'Features', href: '#' },
  { name: 'Marketplace', href: '#' },
  { name: 'Company', href: '#' },
]
const footer = {
  solutions: [
    { name: 'Marketing', href: '#' },
    { name: 'Analytics', href: '#' },
    { name: 'Commerce', href: '#' },
    { name: 'Insights', href: '#' },
  ],
  support: [
    { name: 'Pricing', href: '#' },
    { name: 'Documentation', href: '#' },
    { name: 'Guides', href: '#' },
    { name: 'API Status', href: '#' },
  ],
  company: [
    { name: 'About', href: '#' },
    { name: 'Blog', href: '#' },
    { name: 'Jobs', href: '#' },
    { name: 'Press', href: '#' },
    { name: 'Partners', href: '#' },
  ],
  legal: [
    { name: 'Claim', href: '#' },
    { name: 'Privacy', href: '#' },
    { name: 'Terms', href: '#' },
  ],
}
export function Example() {
  const [mobileMenuOpen, setMobileMenuOpen] = createSignal(false)
  return (
    <>
      {/*
        This example requires updating your template:
        ```
        <html class="h-full">
        <body class="h-full">
        ```
      */}
      <div class="flex min-h-full flex-col">
        <header class="mx-auto w-full max-w-7xl px-6 pt-6 lg:px-8">
          <nav aria-label="Global" class="flex items-center justify-between">
            <div class="flex lg:flex-1">
              <a href="#" class="-m-1.5 p-1.5">
                <span class="sr-only">Your Company</span>
                <img
                  alt=""
                  src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                  class="h-8 dark:hidden"
                />
                <img
                  alt=""
                  src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500"
                  class="h-8 hidden"
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
            <div class="hidden lg:flex lg:gap-x-12">
              {navigation.map((item) => (
                <a href={item.href} class="text-sm/6 font-semibold text-gray-900 dark:text-white">
                  {item.name}
                </a>
              ))}
            </div>
            <div class="hidden lg:flex lg:flex-1 lg:justify-end">
              <a href="#" class="text-sm/6 font-semibold text-gray-900 dark:text-white">
                Log in <span aria-hidden="true">&rarr;</span>
              </a>
            </div>
          </nav>
          <Dialog open={mobileMenuOpen} onClose={setMobileMenuOpen} class="lg:hidden">
            <div class="fixed inset-0 z-50" />
            <DialogPanel class="fixed inset-y-0 right-0 z-50 w-full overflow-y-auto bg-white p-6 sm:max-w-sm sm:ring-1 sm:ring-gray-900/10 dark:bg-gray-900 dark:sm:ring-gray-100/10">
              <div class="flex items-center justify-between">
                <a href="#" class="-m-1.5 p-1.5">
                  <span class="sr-only">Your Company</span>
                  <img
                    alt=""
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                    class="h-8 dark:hidden"
                  />
                  <img
                    alt=""
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500"
                    class="h-8 hidden"
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
                    {navigation.map((item) => (
                      <a
                        href={item.href}
                        class="-mx-3 block rounded-lg px-3 py-2 text-base/7 font-semibold text-gray-900 hover:bg-gray-400/10 dark:text-white dark:hover:bg-white/5"
                      >
                        {item.name}
                      </a>
                    ))}
                  </div>
                  <div class="py-6">
                    <a
                      href="#"
                      class="-mx-3 block rounded-lg px-3 py-2.5 text-base font-semibold text-gray-900 hover:bg-gray-400/10 dark:text-white dark:hover:bg-white/5"
                    >
                      Log in
                    </a>
                  </div>
                </div>
              </div>
            </DialogPanel>
          </Dialog>
        </header>
        <main class="mx-auto flex w-full max-w-7xl flex-auto flex-col justify-center px-6 py-24 sm:py-64 lg:px-8">
          <p class="text-base/8 font-semibold text-indigo-600 dark:text-indigo-400">404</p>
          <h1 class="mt-4 text-5xl font-semibold tracking-tight text-pretty text-gray-900 sm:text-6xl dark:text-white">
            Page not found
          </h1>
          <p class="mt-6 text-lg font-medium text-pretty text-gray-500 sm:text-xl/8 dark:text-gray-400">
            Sorry, we couldn’t find the page you’re looking for.
          </p>
          <div class="mt-10">
            <a href="#" class="text-sm/7 font-semibold text-indigo-600 dark:text-indigo-400">
              <span aria-hidden="true">&larr;</span> Back to home
            </a>
          </div>
        </main>
        <footer aria-labelledby="footer-heading" class="border-t border-gray-200 dark:border-white/10">
          <h2 id="footer-heading" class="sr-only">
            Footer
          </h2>
          <div class="mx-auto max-w-7xl px-6 py-16 sm:py-24 lg:px-8 lg:py-32">
            <div class="xl:grid xl:grid-cols-3 xl:gap-8">
              <img
                alt="Company name"
                src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                class="h-7 dark:hidden"
              />
              <img
                alt="Company name"
                src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500"
                class="h-7 hidden"
              />
              <div class="mt-16 grid grid-cols-2 gap-8 xl:col-span-2 xl:mt-0">
                <div class="md:grid md:grid-cols-2 md:gap-8">
                  <div>
                    <h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white">Solutions</h3>
                    <ul role="list" class="mt-6 space-y-4">
                      {footer.solutions.map((item) => (
                        <li>
                          <a
                            href={item.href}
                            class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white"
                          >
                            {item.name}
                          </a>
                        </li>
                      ))}
                    </ul>
                  </div>
                  <div class="mt-10 md:mt-0">
                    <h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white">Support</h3>
                    <ul role="list" class="mt-6 space-y-4">
                      {footer.support.map((item) => (
                        <li>
                          <a
                            href={item.href}
                            class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white"
                          >
                            {item.name}
                          </a>
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
                <div class="md:grid md:grid-cols-2 md:gap-8">
                  <div>
                    <h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white">Company</h3>
                    <ul role="list" class="mt-6 space-y-4">
                      {footer.company.map((item) => (
                        <li>
                          <a
                            href={item.href}
                            class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white"
                          >
                            {item.name}
                          </a>
                        </li>
                      ))}
                    </ul>
                  </div>
                  <div class="mt-10 md:mt-0">
                    <h3 class="text-sm/6 font-semibold text-gray-900 dark:text-white">Legal</h3>
                    <ul role="list" class="mt-6 space-y-4">
                      {footer.legal.map((item) => (
                        <li>
                          <a
                            href={item.href}
                            class="text-sm/6 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white"
                          >
                            {item.name}
                          </a>
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </footer>
      </div>
    </>
  )
}
