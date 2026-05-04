import type { LocaleCopy } from "@/data/i18n"

type Props = {
  copy: LocaleCopy
}

export const MethodologyTemplate = (props: Props) => {
  return (
    <section class="rounded-2xl border border-white/15 p-5">
      <h3 class="text-xl font-semibold text-white">{props.copy.methodologyTemplate.title}</h3>
      <ul class="mt-4 grid gap-2 text-sm text-slate-200 md:grid-cols-2">
        <li class="rounded-lg border border-white/10 bg-slate-900/60 p-3">
          {props.copy.methodologyTemplate.sourceOfTruth}
        </li>
        <li class="rounded-lg border border-white/10 bg-slate-900/60 p-3">
          {props.copy.methodologyTemplate.fairnessClause}
        </li>
        <li class="rounded-lg border border-white/10 bg-slate-900/60 p-3">
          {props.copy.methodologyTemplate.reproducibility}
        </li>
        <li class="rounded-lg border border-white/10 bg-slate-900/60 p-3">
          {props.copy.methodologyTemplate.confidence}
        </li>
        <li class="rounded-lg border border-white/10 bg-slate-900/60 p-3 md:col-span-2">
          {props.copy.methodologyTemplate.reporting}
        </li>
      </ul>
    </section>
  )
}
