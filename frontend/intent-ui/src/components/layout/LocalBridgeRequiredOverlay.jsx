function LocalBridgeRequiredOverlay(props) {
  return (
    <div class="fixed inset-0 z-[14000] flex items-center justify-center bg-[#070709]/95 px-4 py-6">
      <div class="w-full max-w-md rounded-xl border border-red-500/35 bg-[#16171d] p-4 shadow-2xl">
        <p class="text-[10px] uppercase tracking-wide text-red-300">Local Bridge Required</p>
        <h2 class="mt-1 text-base font-semibold text-neutral-100">{props.error}</h2>
        <p class="mt-2 text-xs text-neutral-400">
          Intent UI requires a local bridge at <code class="text-neutral-200">{props.wsEndpoint}</code>.
        </p>
        <button
          type="button"
          class="mt-3 inline-flex items-center justify-center rounded border border-red-400/50 bg-red-500/10 px-3 py-1.5 text-xs text-red-200 hover:bg-red-500/20"
          onClick={props.onRetry}
          data-testid="retry-local-bridge"
        >
          Retry bridge connection
        </button>
      </div>
    </div>
  );
}

export default LocalBridgeRequiredOverlay;
