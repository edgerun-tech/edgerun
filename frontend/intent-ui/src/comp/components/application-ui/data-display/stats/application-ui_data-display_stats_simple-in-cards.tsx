export interface SimpleInCardsProps {
  class?: string;
}
export function SimpleInCards(props: SimpleInCardsProps): JSX.Element {
  return (
    <div class={props.class || ""}>
<div class="bg-gray-100 dark:bg-gray-900 dark:scheme-dark">
  <div class="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
    <div>
      <h3 class="text-base font-semibold text-gray-900 dark:text-white">Last 30 days</h3>
      <dl class="mt-5 grid grid-cols-1 gap-5 sm:grid-cols-3">
        <div class="overflow-hidden rounded-lg bg-white px-4 py-5 shadow-sm sm:p-6 dark:bg-gray-800/75 dark:inset-ring dark:inset-ring-white/10">
          <dt class="truncate text-sm font-medium text-gray-500 dark:text-gray-400">Total Subscribers</dt>
          <dd class="mt-1 text-3xl font-semibold tracking-tight text-gray-900 dark:text-white">71,897</dd>
        </div>
        <div class="overflow-hidden rounded-lg bg-white px-4 py-5 shadow-sm sm:p-6 dark:bg-gray-800/75 dark:inset-ring dark:inset-ring-white/10">
          <dt class="truncate text-sm font-medium text-gray-500 dark:text-gray-400">Avg. Open Rate</dt>
          <dd class="mt-1 text-3xl font-semibold tracking-tight text-gray-900 dark:text-white">58.16%</dd>
        </div>
        <div class="overflow-hidden rounded-lg bg-white px-4 py-5 shadow-sm sm:p-6 dark:bg-gray-800/75 dark:inset-ring dark:inset-ring-white/10">
          <dt class="truncate text-sm font-medium text-gray-500 dark:text-gray-400">Avg. Click Rate</dt>
          <dd class="mt-1 text-3xl font-semibold tracking-tight text-gray-900 dark:text-white">24.57%</dd>
        </div>
      </dl>
    </div>
  </div>
</div>
    </div>
  );
}
