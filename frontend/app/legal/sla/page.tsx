// SPDX-License-Identifier: Apache-2.0
import { LegalDocumentPage } from '../../../components/layout/legal-document-page'

export default function SlaPage() {
  return (
    <LegalDocumentPage
      title="Service Level Agreement"
      description="Operational targets and support commitments for Edgerun-managed service endpoints."
      effectiveDate="February 24, 2026"
      sections={[
        {
          title: 'Scope',
          paragraphs: [
            'This SLA applies to Edgerun-managed web interfaces and control-plane endpoints identified as covered services.',
            'This SLA does not guarantee performance or availability of third-party networks, wallet providers, user infrastructure, or public blockchain RPC endpoints outside Edgerun control.'
          ]
        },
        {
          title: 'Availability Target',
          paragraphs: [
            'Monthly uptime objective for covered services is 99.5 percent.',
            'Uptime is measured as successful response availability from Edgerun-managed endpoints, excluding planned maintenance and excluded events listed below.'
          ]
        },
        {
          title: 'Support Windows',
          paragraphs: [
            'Incident triage runs continuously for critical production-impacting events.',
            'General support and non-critical requests are handled on a best-effort basis according to published support channel capacity.'
          ]
        },
        {
          title: 'Incident Severity Targets',
          paragraphs: [
            'Severity 1 (critical outage): initial response target within 1 hour.',
            'Severity 2 (major degradation): initial response target within 4 hours.',
            'Severity 3 (minor degradation or advisory issue): initial response target within 1 business day.'
          ]
        },
        {
          title: 'Exclusions',
          paragraphs: [
            'The following are excluded from uptime calculations: scheduled maintenance, force majeure events, upstream provider outages, internet routing failures outside Edgerun control, user misconfiguration, abusive traffic, and incidents caused by unauthorized changes by users.',
            'Features explicitly labeled experimental, preview, or beta are excluded from SLA guarantees unless otherwise stated.'
          ]
        },
        {
          title: 'Credits and Remedies',
          paragraphs: [
            'Where commercial agreements apply, service credits are the sole remedy for verified SLA breaches unless otherwise agreed in writing.',
            'Credit eligibility requires a support request with reproducible evidence submitted within 30 days of the affected period.'
          ]
        },
        {
          title: 'Changes to SLA',
          paragraphs: [
            'We may revise this SLA to reflect architecture, operations, or compliance changes.',
            'Updated SLA terms are effective on publication unless a later date is stated.'
          ]
        }
      ]}
    />
  )
}
