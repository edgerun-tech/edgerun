
const tiers = [
  { name: 'Starter', id: 'tier-starter', href: '#', priceMonthly: '$19', mostPopular: false },
  { name: 'Growth', id: 'tier-growth', href: '#', priceMonthly: '$49', mostPopular: true },
  { name: 'Scale', id: 'tier-scale', href: '#', priceMonthly: '$99', mostPopular: false },
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
function classNames(...classes) {
  return classes.filter(Boolean).join(' ')
}
export function Example() {
  return (
    <div class="bg-white py-24 sm:py-32 dark:bg-gray-900">
      <div class="mx-auto max-w-7xl px-6 lg:px-8">
        <div class="mx-auto max-w-4xl text-center">
          <h2 class="text-base/7 font-semibold text-indigo-600 dark:text-indigo-400">Pricing</h2>
          <p class="mt-2 text-5xl font-semibold tracking-tight text-balance text-gray-900 sm:text-6xl dark:text-white">
            Pricing that grows with you
          </p>
        </div>
        <p class="mx-auto mt-6 max-w-2xl text-center text-lg font-medium text-pretty text-gray-600 sm:text-xl/8 dark:text-gray-400">
          Choose an affordable plan that’s packed with the best features for engaging your audience, creating customer
          loyalty, and driving sales.
        </p>
        {/* xs to lg */}
        <div class="mx-auto mt-12 max-w-md space-y-8 sm:mt-16 lg:hidden">
          {tiers.map((tier) => (
            <section
              class={classNames(
                tier.mostPopular
                  ? 'rounded-xl bg-gray-400/5 inset-ring inset-ring-gray-200 dark:bg-white/5 dark:inset-ring-white/10'
                  : '',
                'p-8',
              )}
            >
              <h3 id={tier.id} class="text-sm/6 font-semibold text-gray-900 dark:text-white">
                {tier.name}
              </h3>
              <p class="mt-2 flex items-baseline gap-x-1 text-gray-900 dark:text-white">
                <span class="text-4xl font-semibold">{tier.priceMonthly}</span>
                <span class="text-sm font-semibold">/month</span>
              </p>
              <a
                href={tier.href}
                aria-describedby={tier.id}
                class={classNames(
                  tier.mostPopular
                    ? 'bg-indigo-600 text-white hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400'
                    : 'text-indigo-600 inset-ring inset-ring-indigo-200 hover:inset-ring-indigo-300 dark:bg-white/10 dark:text-white dark:inset-ring-white/5 dark:hover:bg-white/20 dark:hover:inset-ring-white/5 dark:focus-visible:outline-white/75',
                  'mt-8 block rounded-md px-3 py-2 text-center text-sm/6 font-semibold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:focus-visible:outline-indigo-500',
                )}
              >
                Buy plan
              </a>
              <ul role="list" class="mt-10 space-y-4 text-sm/6 text-gray-900 dark:text-gray-200">
                {sections.map((section) => (
                  <li>
                    <ul role="list" class="space-y-4">
                      {section.features.map((feature) =>
                        feature.tiers[tier.name] ? (
                          <li class="flex gap-x-3">
                            <CheckIcon
                              aria-hidden="true"
                              class="h-6 w-5 flex-none text-indigo-600 dark:text-indigo-400"
                            />
                            <span>
                              {feature.name}{' '}
                              {typeof feature.tiers[tier.name] === 'string' ? (
                                <span class="text-sm/6 text-gray-500 dark:text-gray-400">
                                  ({feature.tiers[tier.name]})
                                </span>
                              ) : null}
                            </span>
                          </li>
                        ) : null,
                      )}
                    </ul>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
        {/* lg+ */}
        <div class="isolate mt-20 hidden lg:block">
          <div class="relative -mx-8">
            {tiers.some((tier) => tier.mostPopular) ? (
              <div class="absolute inset-x-4 inset-y-0 -z-10 flex">
                <div
                  style={{ marginLeft: `${(tiers.findIndex((tier) => tier.mostPopular) + 1) * 25}%` }}
                  aria-hidden="true"
                  class="flex w-1/4 px-4"
                >
                  <div class="w-full rounded-t-xl border-x border-t border-gray-900/10 bg-gray-400/5 dark:border-white/10 dark:bg-gray-800/50" />
                </div>
              </div>
            ) : null}
            <table class="w-full table-fixed border-separate border-spacing-x-8 text-left">
              <caption class="sr-only">Pricing plan comparison</caption>
              <colgroup>
                <col class="w-1/4" />
                <col class="w-1/4" />
                <col class="w-1/4" />
                <col class="w-1/4" />
              </colgroup>
              <thead>
                <tr>
                  <td />
                  {tiers.map((tier) => (
                    <th scope="col" class="px-6 pt-6 xl:px-8 xl:pt-8">
                      <div class="text-sm/7 font-semibold text-gray-900 dark:text-white">{tier.name}</div>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                <tr>
                  <th scope="row">
                    <span class="sr-only">Price</span>
                  </th>
                  {tiers.map((tier) => (
                    <td class="px-6 pt-2 xl:px-8">
                      <div class="flex items-baseline gap-x-1 text-gray-900 dark:text-white">
                        <span class="text-4xl font-semibold">{tier.priceMonthly}</span>
                        <span class="text-sm/6 font-semibold">/month</span>
                      </div>
                      <a
                        href={tier.href}
                        class={classNames(
                          tier.mostPopular
                            ? 'bg-indigo-600 text-white hover:bg-indigo-500 dark:bg-indigo-500 dark:hover:bg-indigo-400'
                            : 'text-indigo-600 inset-ring inset-ring-indigo-200 hover:inset-ring-indigo-300 dark:bg-white/10 dark:text-white dark:inset-ring-white/5 dark:hover:bg-white/20 dark:hover:inset-ring-white/5 dark:focus-visible:outline-white/75',
                          'mt-8 block rounded-md px-3 py-2 text-center text-sm/6 font-semibold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:focus-visible:outline-indigo-500',
                        )}
                      >
                        Buy plan
                      </a>
                    </td>
                  ))}
                </tr>
                {sections.map((section, sectionIdx) => (
                  <Fragment>
                    <tr>
                      <th
                        scope="colgroup"
                        colSpan={4}
                        class={classNames(
                          sectionIdx === 0 ? 'pt-8' : 'pt-16',
                          'pb-4 text-sm/6 font-semibold text-gray-900 dark:text-white',
                        )}
                      >
                        {section.name}
                        <div class="absolute inset-x-8 mt-4 h-px bg-gray-900/10 dark:bg-white/10" />
                      </th>
                    </tr>
                    {section.features.map((feature) => (
                      <tr>
                        <th scope="row" class="py-4 text-sm/6 font-normal text-gray-900 dark:text-gray-200">
                          {feature.name}
                          <div class="absolute inset-x-8 mt-4 h-px bg-gray-900/5 dark:bg-white/5" />
                        </th>
                        {tiers.map((tier) => (
                          <td class="px-6 py-4 xl:px-8">
                            {typeof feature.tiers[tier.name] === 'string' ? (
                              <div class="text-center text-sm/6 text-gray-500 dark:text-gray-400">
                                {feature.tiers[tier.name]}
                              </div>
                            ) : (
                              <>
                                {feature.tiers[tier.name] === true ? (
                                  <CheckIcon
                                    aria-hidden="true"
                                    class="mx-auto size-5 text-indigo-600 dark:text-indigo-400"
                                  />
                                ) : (
                                  <MinusIcon
                                    aria-hidden="true"
                                    class="mx-auto size-5 text-gray-400 dark:text-gray-500"
                                  />
                                )}
                                <span class="sr-only">
                                  {feature.tiers[tier.name] === true ? 'Included' : 'Not included'} in {tier.name}
                                </span>
                              </>
                            )}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  )
}
