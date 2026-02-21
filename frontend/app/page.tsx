import { HeroSection } from '../components/landing/hero-section'
import { FeaturesSection } from '../components/landing/features-section'
import { TerminalDemo } from '../components/landing/terminal-demo'
import { UseCases } from '../components/landing/use-cases'
import { CTASection } from '../components/landing/cta-section'
import { PageShell } from '../components/layout/page-shell'

export default function HomePage() {
  return (
    <PageShell>
      <HeroSection />
      <FeaturesSection />
      <TerminalDemo />
      <UseCases />
      <CTASection />
    </PageShell>
  )
}
