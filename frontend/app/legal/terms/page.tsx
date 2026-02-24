// SPDX-License-Identifier: Apache-2.0
import { LegalDocumentPage } from '../../../components/layout/legal-document-page'

export default function TermsPage() {
  return (
    <LegalDocumentPage
      title="Terms of Service"
      description="Conditions for using Edgerun website and platform tooling."
      effectiveDate="February 24, 2026"
      sections={[
        {
          title: 'Acceptance of Terms',
          paragraphs: [
            'By accessing or using Edgerun services, you agree to these Terms of Service and any policies referenced by them.',
            'If you do not agree to these terms, do not use the service.'
          ]
        },
        {
          title: 'Service Description',
          paragraphs: [
            'Edgerun provides tooling for deterministic compute workflows, including job submission, scheduling interfaces, worker coordination, and related observability views.',
            'Service features may evolve over time. We may add, remove, or modify features to improve safety, reliability, and protocol compatibility.'
          ]
        },
        {
          title: 'User Responsibilities',
          paragraphs: [
            'You are responsible for all activity initiated from your wallet sessions, credentials, or operator environment.',
            'You must provide lawful inputs and must not use the service for unauthorized access, malware deployment, fraud, or other abusive behavior.',
            'You are responsible for validating outputs and for operational decisions made using platform data.'
          ]
        },
        {
          title: 'On-Chain and Financial Risk',
          paragraphs: [
            'Certain actions may produce on-chain transactions, public records, fees, or irreversible state changes.',
            'You are solely responsible for understanding transaction consequences, fee exposure, and wallet key management before submitting jobs or signing messages.',
            'Edgerun does not provide financial, investment, or legal advice.'
          ]
        },
        {
          title: 'Availability and Warranties',
          paragraphs: [
            'The service is provided on an "as is" and "as available" basis without warranties of uninterrupted availability, fitness for a particular purpose, or non-infringement.',
            'We do not warrant that the service will be error-free, continuously available, or suitable for every workload.',
            'Service-level objectives, when offered, are defined separately in the Service Level Agreement.'
          ]
        },
        {
          title: 'Limitation of Liability',
          paragraphs: [
            'To the maximum extent permitted by law, Edgerun and its contributors are not liable for indirect, incidental, special, consequential, or punitive damages arising from service use.',
            'Total liability for claims related to service use is limited to amounts paid by you to Edgerun for the specific service giving rise to the claim, if any.'
          ]
        },
        {
          title: 'Termination and Updates',
          paragraphs: [
            'We may suspend or terminate access for abuse, legal compliance, security incidents, or violations of these terms.',
            'We may update these terms from time to time. Continued use after updates constitutes acceptance of the revised terms.'
          ]
        }
      ]}
    />
  )
}
