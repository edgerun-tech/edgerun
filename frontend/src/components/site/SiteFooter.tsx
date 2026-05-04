export const SiteFooter = () => {
  return (
    <footer class="mt-16 border-t border-white/10 bg-slate-950/55 py-10">
      <div class="mx-auto flex max-w-6xl flex-col gap-3 px-4 text-sm text-slate-400 sm:px-6">
        <p>© {new Date().getFullYear()} EdgeRun technical publications.</p>
        <p>
          Evidence-first updates. Every published number includes source links,
          run manifests, and reproducibility templates.
        </p>
      </div>
    </footer>
  )
}
