export interface WithDescriptionProps {
  class?: string;
}
export function WithDescription(props: WithDescriptionProps): JSX.Element {
  return (
    <div class={props.class || ""}>
<div class="bg-white dark:bg-gray-900 dark:scheme-dark">
  <div class="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
    <div class="border-b border-gray-200 pb-5 dark:border-white/10">
      <h3 class="text-base font-semibold text-gray-900 dark:text-white">Job Postings</h3>
      <p class="mt-2 max-w-4xl text-sm text-gray-500 dark:text-gray-400">Workcation is a property rental website. Etiam ullamcorper massa viverra consequat, consectetur id nulla tempus. Fringilla egestas justo massa purus sagittis malesuada.</p>
    </div>
  </div>
</div>
    </div>
  );
}
