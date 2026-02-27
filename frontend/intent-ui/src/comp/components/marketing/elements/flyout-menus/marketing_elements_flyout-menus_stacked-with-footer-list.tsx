


const resources = [
  { name: 'Help center', description: 'Get all of your questions answered', href: '#', icon: LifebuoyIcon },
  { name: 'Guides', description: 'Learn how to maximize our platform', href: '#', icon: BookmarkSquareIcon },
  { name: 'Events', description: 'See meet-ups and other events near you', href: '#', icon: CalendarDaysIcon },
]
const recentPosts = [
  { id: 1, title: 'Boost your conversion rate', href: '#', date: 'Mar 5, 2023', datetime: '2023-03-05' },
  {
    id: 2,
    title: 'How to use search engine optimization to drive traffic to your site',
    href: '#',
    date: 'Feb 25, 2023',
    datetime: '2023-02-25',
  },
  { id: 3, title: 'Improve your customer experience', href: '#', date: 'Feb 21, 2023', datetime: '2023-02-21' },
]
export function Example() {
  return (
    <Popover class="relative">
      <PopoverButton class="inline-flex items-center gap-x-1 text-sm/6 font-semibold text-gray-900 dark:text-white">
        <span>Resources</span>
        <ChevronDownIcon aria-hidden="true" class="size-5" />
      </PopoverButton>
      <PopoverPanel
        transition
        class="absolute left-1/2 z-10 mt-5 flex w-screen max-w-max -translate-x-1/2 bg-transparent px-4 transition data-closed:translate-y-1 data-closed:opacity-0 data-enter:duration-200 data-enter:ease-out data-leave:duration-150 data-leave:ease-in"
      >
        <div class="w-screen max-w-md flex-auto overflow-hidden rounded-3xl bg-white text-sm/6 shadow-lg outline-1 outline-gray-900/5 dark:bg-gray-900 dark:shadow-none dark:-outline-offset-1 dark:outline-white/10">
          <div class="p-4">
            {resources.map((item) => (
              <div
                class="group relative flex gap-x-6 rounded-lg p-4 hover:bg-gray-50 dark:hover:bg-white/5"
              >
                <div class="mt-1 flex size-11 flex-none items-center justify-center rounded-lg bg-gray-50 group-hover:bg-white dark:bg-gray-700/50 dark:group-hover:bg-gray-700">
                  <item.icon
                    aria-hidden="true"
                    class="size-6 text-gray-600 group-hover:text-indigo-600 dark:text-gray-400 dark:group-hover:text-white"
                  />
                </div>
                <div>
                  <a href={item.href} class="font-semibold text-gray-900 dark:text-white">
                    {item.name}
                    <span class="absolute inset-0" />
                  </a>
                  <p class="mt-1 text-gray-600 dark:text-gray-400">{item.description}</p>
                </div>
              </div>
            ))}
          </div>
          <div class="bg-gray-50 p-8 dark:bg-gray-800/50">
            <div class="flex justify-between">
              <h3 class="text-sm/6 font-semibold text-gray-500 dark:text-gray-400">Recent posts</h3>
              <a href="#" class="text-sm/6 font-semibold text-indigo-600 dark:text-indigo-400">
                See all <span aria-hidden="true">&rarr;</span>
              </a>
            </div>
            <ul role="list" class="mt-6 space-y-6">
              {recentPosts.map((post) => (
                <li class="relative">
                  <time dateTime={post.datetime} class="block text-xs/6 text-gray-600 dark:text-gray-400">
                    {post.date}
                  </time>
                  <a href={post.href} class="block truncate text-sm/6 font-semibold text-gray-900 dark:text-white">
                    {post.title}
                    <span class="absolute inset-0" />
                  </a>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </PopoverPanel>
    </Popover>
  )
}
