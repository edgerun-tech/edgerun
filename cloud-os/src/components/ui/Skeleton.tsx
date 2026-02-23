import { Motion } from "solid-motionone";

interface WindowSkeletonProps {
  title?: string;
  showTabs?: boolean;
}

export function WindowSkeleton(props: WindowSkeletonProps) {
  return (
    <div class="h-full flex flex-col bg-[#1a1a1a]">
      <div class="h-10 flex items-center gap-2 px-4" style={{ background: "linear-gradient(to bottom, #2d2d2d, #252525)" }}>
        <div class="flex gap-2">
          <div class="w-3 h-3 rounded-full bg-neutral-600" />
          <div class="w-3 h-3 rounded-full bg-neutral-600" />
          <div class="w-3 h-3 rounded-full bg-neutral-600" />
        </div>
        <div class="flex-1 flex justify-center">
          <div class="w-[100px] h-[14px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
        </div>
      </div>
      <div class="flex-1 p-4 space-y-4">
        {props.showTabs && (
          <div class="flex gap-2">
            <div class="w-[80px] h-[28px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
            <div class="w-[80px] h-[28px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
            <div class="w-[80px] h-[28px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          </div>
        )}
        <div class="space-y-2">
          <div class="w-full h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          <div class="w-full h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          <div class="w-[90%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          <div class="w-[95%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          <div class="w-[85%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          <div class="w-[70%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
        </div>
        <div class="grid grid-cols-2 gap-3 mt-4">
          <div class="bg-neutral-800/50 rounded-lg p-4">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-[40px] h-[40px] rounded-full bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 animate-pulse" />
              <div class="flex-1 space-y-2">
                <div class="w-[60%] h-[14px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
                <div class="w-[40%] h-[12px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
              </div>
            </div>
            <div class="space-y-2">
              <div class="w-full h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
              <div class="w-[90%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
            </div>
          </div>
          <div class="bg-neutral-800/50 rounded-lg p-4">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-[40px] h-[40px] rounded-full bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 animate-pulse" />
              <div class="flex-1 space-y-2">
                <div class="w-[60%] h-[14px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
                <div class="w-[40%] h-[12px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
              </div>
            </div>
            <div class="space-y-2">
              <div class="w-full h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
              <div class="w-[90%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function EditorSkeleton() {
  return (
    <div class="h-full flex flex-col bg-[#1e1e1e]">
      <div class="h-9 flex items-center gap-1 px-2 border-b border-neutral-700">
        {["src", "components", "utils"].map((tab) => (
          <div class="flex items-center gap-2 px-3 h-full bg-[#2d2d2d] border-t-2 border-t-blue-500">
            <div class="w-[40px] h-[12px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          </div>
        ))}
      </div>
      <div class="flex-1 flex">
        <div class="w-48 border-r border-neutral-700 p-2 space-y-2">
          {Array.from({ length: 8 }).map(() => (
            <div class="flex items-center gap-2">
              <div class="w-[16px] h-[16px] rounded bg-neutral-600" />
              <div class="w-[60px] h-[12px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
            </div>
          ))}
        </div>
        <div class="flex-1 p-4 space-y-1">
          {Array.from({ length: 15 }).map((_, i) => (
            <div class="flex items-center gap-3">
              <div class="w-[30px] h-[12px] bg-neutral-700/50 rounded" />
              <div class={`h-[12px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse ${i === 0 ? 'w-[40%]' : i === 1 ? 'w-[60%]' : 'w-[80%]'}`} />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function FileManagerSkeleton() {
  return (
    <div class="h-full flex flex-col bg-[#1a1a1a]">
      <div class="h-12 flex items-center gap-2 px-4 border-b border-neutral-700">
        <div class="w-[200px] h-[28px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
        <div class="flex-1" />
        <div class="w-[28px] h-[28px] rounded bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 animate-pulse" />
        <div class="w-[28px] h-[28px] rounded bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 animate-pulse" />
      </div>
      <div class="p-2">
        <div class="space-y-2 mb-2">
          <div class="w-full h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          <div class="w-[90%] h-[16px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
        </div>
      </div>
      <div class="flex-1 p-2 space-y-1">
        {Array.from({ length: 12 }).map(() => (
          <div class="flex items-center gap-3 p-2 hover:bg-neutral-800 rounded">
            <div class="w-[24px] h-[24px] rounded bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 animate-pulse" />
            <div class="w-[40%] h-[14px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
            <div class="flex-1" />
            <div class="w-[20%] h-[12px] bg-gradient-to-r from-neutral-700 via-neutral-600 to-neutral-700 rounded animate-pulse" />
          </div>
        ))}
      </div>
    </div>
  );
}

export function LoadingDots(props: { class?: string }) {
  return (
    <div class={`flex items-center gap-1 ${props.class || ""}`}>
      {[0, 1, 2].map((i) => (
        <Motion.div
          animate={{ opacity: [0.3, 1, 0.3] }}
          transition={{ duration: 0.6, repeat: Infinity, delay: i * 0.15 }}
          class="w-2 h-2 bg-blue-500 rounded-full"
        />
      ))}
    </div>
  );
}
