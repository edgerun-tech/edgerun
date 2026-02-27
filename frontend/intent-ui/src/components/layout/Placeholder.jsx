import { TbOutlineHourglass } from "solid-icons/tb";
function Placeholder(props) {
  return <div class="h-full flex flex-col items-center justify-center bg-[#1a1a1a] text-neutral-400 p-8">
      <TbOutlineHourglass size={64} class="mb-6 opacity-50" />
      <h2 class="text-2xl font-semibold text-white mb-2">{props.feature}</h2>
      <p class="text-lg mb-6">Coming Soon</p>
      <p class="text-sm text-neutral-500 text-center max-w-md">
        This feature is under development. Check back later for updates!
      </p>
    </div>;
}
var stdin_default = Placeholder;
export {
  Placeholder,
  stdin_default as default
};
