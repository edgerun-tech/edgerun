// SPDX-License-Identifier: Apache-2.0
import { LegalDocumentPage } from '../../../components/layout/legal-document-page'

export default function PrivacyPage() {
  return (
    <LegalDocumentPage
      title="Privacy Policy"
      description="How Edgerun handles personal data across website usage, job orchestration workflows, and operational telemetry."
      effectiveDate="February 24, 2026"
      sections={[
        {
          title: 'Scope',
          paragraphs: [
            'This Privacy Policy applies to the Edgerun website, user-facing dashboards, job orchestration interfaces, and related support channels controlled by the Edgerun project.',
            'This policy covers data collected directly by Edgerun services. Public blockchain data is inherently public and is not controlled by this policy once written on-chain.'
          ]
        },
        {
          title: 'Data We Collect',
          paragraphs: [
            'Account and identity data: wallet public keys, session state, and optional profile preferences needed to operate authenticated features.',
            'Operational data: job metadata, scheduler and worker coordination events, routing metadata, and client diagnostics needed to run and secure the platform.',
            'Website usage data: page interactions, browser metadata, and performance telemetry used to maintain reliability and detect abuse.'
          ]
        },
        {
          title: 'How We Use Data',
          paragraphs: [
            'We use collected data to provide core platform functionality, including scheduling jobs, routing workers, showing execution status, and enforcing protocol safety controls.',
            'We use telemetry and security signals to detect fraud, enforce abuse controls, investigate incidents, and improve service reliability.',
            'We do not sell personal data. We process data for service operation, security, compliance, and support.'
          ]
        },
        {
          title: 'Data Sharing',
          paragraphs: [
            'We may share data with infrastructure providers and service processors that support hosting, observability, storage, and incident response, under contractual confidentiality terms.',
            'We may disclose data when required by law, regulation, legal process, or to protect the safety and rights of users and the platform.',
            'On-chain records and signatures may be publicly visible by design and can be accessed by any network participant.'
          ]
        },
        {
          title: 'Retention and Security',
          paragraphs: [
            'We retain data only as long as needed for platform operation, legal obligations, dispute handling, and security investigations.',
            'Retention duration varies by data type and operational need. Security logs may be retained longer where required for abuse prevention and compliance.',
            'We apply reasonable technical and organizational safeguards, but no internet system can guarantee absolute security.'
          ]
        },
        {
          title: 'Your Choices',
          paragraphs: [
            'You may disconnect wallet sessions and stop using the service at any time, but public on-chain records cannot be erased by Edgerun.',
            'Where applicable, you may request access, correction, or deletion of off-chain personal data by contacting project support channels.',
            'If policy terms materially change, updated terms and effective dates will be published on this page.'
          ]
        }
      ]}
    />
  )
}
