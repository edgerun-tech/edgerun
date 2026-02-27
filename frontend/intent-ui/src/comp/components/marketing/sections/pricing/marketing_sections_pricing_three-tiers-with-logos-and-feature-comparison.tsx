

const tiers = [
  {
    name: 'Starter',
    description: 'Everything you need to get started.',
    priceMonthly: '$19',
    href: '#',
    highlights: [
      { description: 'Custom domains' },
      { description: 'Edge content delivery' },
      { description: 'Advanced analytics' },
      { description: 'Quarterly workshops', disabled: true },
      { description: 'Single sign-on (SSO)', disabled: true },
      { description: 'Priority phone support', disabled: true },
    ],
  },
  {
    name: 'Growth',
    description: 'All the extras for your growing team.',
    priceMonthly: '$49',
    href: '#',
    highlights: [
      { description: 'Custom domains' },
      { description: 'Edge content delivery' },
      { description: 'Advanced analytics' },
      { description: 'Quarterly workshops' },
      { description: 'Single sign-on (SSO)', disabled: true },
      { description: 'Priority phone support', disabled: true },
    ],
  },
  {
    name: 'Scale',
    description: 'Added flexibility at scale.',
    priceMonthly: '$99',
    href: '#',
    highlights: [
      { description: 'Custom domains' },
      { description: 'Edge content delivery' },
      { description: 'Advanced analytics' },
      { description: 'Quarterly workshops' },
      { description: 'Single sign-on (SSO)' },
      { description: 'Priority phone support' },
    ],
  },
]
const sections = [
  {
    name: 'Features',
    features: [
      { name: 'Edge content delivery', tiers: { Starter: true, Growth: true, Scale: true } },
      { name: 'Custom domains', tiers: { Starter: '1', Growth: '3', Scale: 'Unlimited' } },
      { name: 'Team members', tiers: { Starter: '3', Growth: '20', Scale: 'Unlimited' } },
      { name: 'Single sign-on (SSO)', tiers: { Starter: false, Growth: false, Scale: true } },
    ],
  },
  {
    name: 'Reporting',
    features: [
      { name: 'Advanced analytics', tiers: { Starter: true, Growth: true, Scale: true } },
      { name: 'Basic reports', tiers: { Starter: false, Growth: true, Scale: true } },
      { name: 'Professional reports', tiers: { Starter: false, Growth: false, Scale: true } },
      { name: 'Custom report builder', tiers: { Starter: false, Growth: false, Scale: true } },
    ],
  },
  {
    name: 'Support',
    features: [
      { name: '24/7 online support', tiers: { Starter: true, Growth: true, Scale: true } },
      { name: 'Quarterly workshops', tiers: { Starter: false, Growth: true, Scale: true } },
      { name: 'Priority phone support', tiers: { Starter: false, Growth: false, Scale: true } },
      { name: '1:1 onboarding tour', tiers: { Starter: false, Growth: false, Scale: true } },
    ],
  },
]
export function Example() {
  return (
    <div class="bg-white py-24 sm:py-32 dark:bg-gray-900">
      <div class="mx-auto max-w-4xl px-6 max-lg:text-center lg:max-w-7xl lg:px-8">
        <h1 class="text-5xl font-semibold tracking-tight text-balance text-gray-950 sm:text-6xl lg:text-pretty dark:text-white">
          Pricing that grows with your team size
        </h1>
        <p class="mt-6 max-w-2xl text-lg font-medium text-pretty text-gray-600 max-lg:mx-auto sm:text-xl/8 dark:text-gray-400">
          Choose an affordable plan that’s packed with the best features for engaging your audience, creating customer
          loyalty, and driving sales.
        </p>
      </div>
      <div class="relative pt-16 sm:pt-24">
        <div class="absolute inset-x-0 top-48 bottom-0 bg-[radial-gradient(circle_at_center_center,#7775D6,#592E71,#030712_70%)] lg:bg-[radial-gradient(circle_at_center_150%,#7775D6,#592E71,#030712_70%)] dark:bg-[radial-gradient(circle_at_center_center,#7775D680,#592E7180,transparent_70%)] dark:lg:bg-[radial-gradient(circle_at_center_150%,#7775D680,#592E7180,transparent_70%)]" />
        <div class="relative mx-auto max-w-2xl px-6 lg:max-w-7xl lg:px-8">
          <div class="grid grid-cols-1 gap-10 lg:grid-cols-3">
            {tiers.map((tier) => (
              <div
                class="-m-2 grid grid-cols-1 rounded-4xl bg-white/2.5 shadow-[inset_0_0_2px_1px_#ffffff4d] ring-1 ring-black/5 max-lg:mx-auto max-lg:w-full max-lg:max-w-md dark:shadow-[inset_0_0_2px_1px_#ffffff32]"
              >
                <div class="grid grid-cols-1 rounded-4xl p-2 shadow-md shadow-black/5 dark:shadow-none">
                  <div class="rounded-3xl bg-white p-10 pb-9 shadow-2xl ring-1 ring-black/5 dark:bg-gray-800 dark:shadow-none dark:ring-white/10">
                    <h2 class="text-sm font-semibold text-indigo-600 dark:text-indigo-400">
                      {tier.name} <span class="sr-only">plan</span>
                    </h2>
                    <p class="mt-2 text-sm/6 text-pretty text-gray-600 dark:text-gray-300">{tier.description}</p>
                    <div class="mt-8 flex items-center gap-4">
                      <div class="text-5xl font-semibold text-gray-950 dark:text-white">{tier.priceMonthly}</div>
                      <div class="text-sm text-gray-600 dark:text-gray-400">
                        <p>USD</p>
                        <p>per month</p>
                      </div>
                    </div>
                    <div class="mt-8">
                      <a
                        href={tier.href}
                        aria-label={`Start a free trial on the ${tier.name} plan`}
                        class="inline-block rounded-md bg-indigo-600 px-3.5 py-2 text-center text-sm/6 font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:shadow-none dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-400"
                      >
                        Start a free trial
                      </a>
                    </div>
                    <div class="mt-8">
                      <h3 class="text-sm/6 font-medium text-gray-950 dark:text-white">Start selling with:</h3>
                      <ul class="mt-3 space-y-3">
                        {tier.highlights.map((highlight) => (
                          <li
                            data-disabled={highlight.disabled}
                            class="group flex items-start gap-4 text-sm/6 text-gray-600 data-disabled:text-gray-400 dark:text-gray-300 dark:data-disabled:text-gray-500"
                          >
                            <span class="inline-flex h-6 items-center">
                              <PlusIcon
                                aria-hidden="true"
                                class="size-4 fill-gray-400 group-data-disabled:fill-gray-300 dark:fill-gray-500 dark:group-data-disabled:fill-gray-600"
                              />
                            </span>
                            {highlight.disabled ? <span class="sr-only">Not included:</span> : null}
                            {highlight.description}
                          </li>
                        ))}
                      </ul>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
          <div class="flex justify-between py-16 opacity-60 max-sm:mx-auto max-sm:max-w-md max-sm:flex-wrap max-sm:justify-evenly max-sm:gap-x-4 max-sm:gap-y-4 sm:py-24">
            <img
              alt="Transistor"
              src="https://tailwindcss.com/plus-assets/img/logos/158x48/transistor-logo-white.svg"
              class="h-9 max-sm:mx-auto sm:h-8 lg:h-12"
            />
            <img
              alt="Laravel"
              src="https://tailwindcss.com/plus-assets/img/logos/158x48/laravel-logo-white.svg"
              class="h-9 max-sm:mx-auto sm:h-8 lg:h-12"
            />
            <img
              alt="Tuple"
              src="https://tailwindcss.com/plus-assets/img/logos/158x48/tuple-logo-white.svg"
              class="h-9 max-sm:mx-auto sm:h-8 lg:h-12"
            />
            <img
              alt="SavvyCal"
              src="https://tailwindcss.com/plus-assets/img/logos/158x48/savvycal-logo-white.svg"
              class="h-9 max-sm:mx-auto sm:h-8 lg:h-12"
            />
            <img
              alt="Statamic"
              src="https://tailwindcss.com/plus-assets/img/logos/158x48/statamic-logo-white.svg"
              class="h-9 max-sm:mx-auto sm:h-8 lg:h-12"
            />
          </div>
        </div>
      </div>
      <div class="mx-auto max-w-2xl px-6 pt-16 sm:pt-24 lg:max-w-7xl lg:px-8">
        <table class="w-full text-left max-sm:hidden">
          <caption class="sr-only">Pricing plan comparison</caption>
          <colgroup>
            <col class="w-2/5" />
            <col class="w-1/5" />
            <col class="w-1/5" />
            <col class="w-1/5" />
          </colgroup>
          <thead>
            <tr>
              <td class="p-0" />
              {tiers.map((tier) => (
                <th scope="col" class="p-0">
                  <div class="text-sm font-semibold text-indigo-600 dark:text-indigo-400">
                    {tier.name} <span class="sr-only">plan</span>
                  </div>
                </th>
              ))}
            </tr>
            <tr>
              <th class="p-0" />
              {tiers.map((tier) => (
                <td class="px-0 pt-3 pb-0">
                  <a
                    href={tier.href}
                    aria-label={`Get started with the ${tier.name} plan`}
                    class="inline-block rounded-md bg-white px-2.5 py-1.5 text-sm font-semibold text-gray-900 shadow-xs inset-ring-1 inset-ring-gray-300 hover:bg-gray-50 dark:bg-white/10 dark:text-white dark:shadow-none dark:inset-ring dark:inset-ring-white/5 dark:hover:bg-white/20"
                  >
                    Get started
                  </a>
                </td>
              ))}
            </tr>
          </thead>
          {sections.map((section) => (
            <tbody class="group">
              <tr>
                <th scope="colgroup" colSpan={4} class="px-0 pt-10 pb-0 group-first-of-type:pt-5">
                  <div class="-mx-4 rounded-lg bg-gray-50 px-4 py-3 text-sm/6 font-semibold text-gray-950 dark:bg-gray-800/50 dark:text-white">
                    {section.name}
                  </div>
                </th>
              </tr>
              {section.features.map((feature) => (
                <tr class="border-b border-gray-100 last:border-none dark:border-white/10">
                  <th scope="row" class="px-0 py-4 text-sm/6 font-normal text-gray-600 dark:text-gray-300">
                    {feature.name}
                  </th>
                  {tiers.map((tier) => (
                    <td class="p-4 max-sm:text-center">
                      {typeof feature.tiers[tier.name] === 'string' ? (
                        <>
                          <span class="sr-only">{tier.name} includes:</span>
                          <span class="text-sm/6 text-gray-950 dark:text-white">{feature.tiers[tier.name]}</span>
                        </>
                      ) : (
                        <>
                          {feature.tiers[tier.name] === true ? (
                            <CheckIcon
                              aria-hidden="true"
                              class="inline-block size-4 fill-green-600 dark:fill-green-500"
                            />
                          ) : (
                            <MinusIcon
                              aria-hidden="true"
                              class="inline-block size-4 fill-gray-400 dark:fill-gray-500"
                            />
                          )}
                          <span class="sr-only">
                            {feature.tiers[tier.name] === true
                              ? `Included in ${tier.name}`
                              : `Not included in ${tier.name}`}
                          </span>
                        </>
                      )}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          ))}
        </table>
        <TabGroup class="sm:hidden">
          <TabList class="flex">
            {tiers.map((tier) => (
              <Tab
                class="w-1/3 border-b border-gray-100 py-4 text-base/8 font-medium text-indigo-600 not-focus-visible:focus:outline-none data-selected:border-indigo-600 dark:border-white/10 dark:text-indigo-400 dark:data-selected:border-indigo-400"
              >
                {tier.name}
              </Tab>
            ))}
          </TabList>
          <TabPanels as={Fragment}>
            {tiers.map((tier) => (
              <TabPanel class="focus:outline-none">
                <a
                  href={tier.href}
                  class="mt-8 block rounded-md bg-white px-3.5 py-2.5 text-center text-sm font-semibold text-gray-900 shadow-xs inset-ring ring-gray-300 hover:bg-gray-50 dark:bg-white/10 dark:text-white dark:shadow-none dark:ring-transparent dark:inset-ring-white/5 dark:hover:bg-white/20"
                >
                  Get started
                </a>
                {sections.map((section) => (
                  <Fragment>
                    <div class="-mx-6 mt-10 rounded-lg bg-gray-50 px-6 py-3 text-sm/6 font-semibold text-gray-950 group-first-of-type:mt-5 dark:bg-gray-800/50 dark:text-white">
                      {section.name}
                    </div>
                    <dl>
                      {section.features.map((feature) => (
                        <div
                          class="grid grid-cols-2 border-b border-gray-100 py-4 last:border-none dark:border-white/10"
                        >
                          <dt class="text-sm/6 font-normal text-gray-600 dark:text-gray-300">{feature.name}</dt>
                          <dd class="text-center">
                            {typeof feature.tiers[tier.name] === 'string' ? (
                              <span class="text-sm/6 text-gray-950 dark:text-white">
                                {feature.tiers[tier.name]}
                              </span>
                            ) : (
                              <>
                                {feature.tiers[tier.name] === true ? (
                                  <CheckIcon
                                    aria-hidden="true"
                                    class="inline-block size-4 fill-green-600 dark:fill-green-500"
                                  />
                                ) : (
                                  <MinusIcon
                                    aria-hidden="true"
                                    class="inline-block size-4 fill-gray-400 dark:fill-gray-500"
                                  />
                                )}
                                <span class="sr-only">{feature.tiers[tier.name] === true ? 'Yes' : 'No'}</span>
                              </>
                            )}
                          </dd>
                        </div>
                      ))}
                    </dl>
                  </Fragment>
                ))}
              </TabPanel>
            ))}
          </TabPanels>
        </TabGroup>
      </div>
    </div>
  )
}
