// SPDX-License-Identifier: Apache-2.0
import { createSignal } from 'solid-js'
import { Input } from './ui/input'
import { Button } from './ui/button'

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

export function FooterLeadForm() {
  const [email, setEmail] = createSignal('')
  const [submitting, setSubmitting] = createSignal(false)
  const [feedback, setFeedback] = createSignal('')
  const [error, setError] = createSignal('')

  const onSubmit = async (event: SubmitEvent) => {
    event.preventDefault()
    if (submitting()) return

    const normalizedEmail = email().trim().toLowerCase()
    if (!EMAIL_RE.test(normalizedEmail)) {
      setError('Enter a valid email address.')
      setFeedback('')
      return
    }

    setSubmitting(true)
    setError('')
    setFeedback('')
    try {
      const response = await fetch('/api/lead', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          email: normalizedEmail,
          sourcePath: typeof window === 'undefined' ? '/' : window.location.pathname
        })
      })

      if (!response.ok) {
        setError('Could not subscribe right now. Try again in a minute.')
        return
      }

      setFeedback('Thanks. You are on the release updates list.')
      setEmail('')
    } catch {
      setError('Could not subscribe right now. Try again in a minute.')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div class="rounded-lg border border-border bg-background/60 p-4">
      <form class="flex flex-col gap-3 sm:flex-row sm:items-center" onSubmit={onSubmit}>
        <div class="flex-1">
          <p class="text-sm font-semibold">Get release updates</p>
          <p class="text-xs text-muted-foreground">Product and protocol notes by email.</p>
        </div>
        <div class="flex w-full gap-2 sm:w-auto">
          <Input
            data-testid="footer-lead-email"
            placeholder="you@company.com"
            type="email"
            value={email()}
            onInput={(event: Event & { currentTarget: HTMLInputElement }) => setEmail(event.currentTarget.value)}
            required
          />
          <Button data-testid="footer-lead-submit" variant="outline" type="submit" disabled={submitting()}>
            {submitting() ? 'Submitting...' : 'Subscribe'}
          </Button>
        </div>
      </form>
      <p data-testid="footer-lead-feedback" class="mt-2 min-h-[1.25rem] text-xs text-muted-foreground">
        {error() || feedback()}
      </p>
    </div>
  )
}
