/**
 * Placeholder Component
 * Generic "Coming Soon" placeholder for missing features
 */

import { TbOutlineHourglass } from 'solid-icons/tb';

interface PlaceholderProps {
  feature: string;
}

export function Placeholder(props: PlaceholderProps) {
  return (
    <div class="h-full flex flex-col items-center justify-center bg-[#1a1a1a] text-neutral-400 p-8">
      <TbOutlineHourglass size={64} class="mb-6 opacity-50" />
      <h2 class="text-2xl font-semibold text-white mb-2">{props.feature}</h2>
      <p class="text-lg mb-6">Coming Soon</p>
      <p class="text-sm text-neutral-500 text-center max-w-md">
        This feature is under development. Check back later for updates!
      </p>
    </div>
  );
}

export default Placeholder;
