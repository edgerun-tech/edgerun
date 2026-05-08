"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { AlertCircle, Download, ExternalLink, LogOut, Mail, Phone, RefreshCw, Save, UserRound, UsersRound } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth } from "@/hooks/use-auth"

type GooglePerson = {
  resourceName: string
  names?: Array<{ displayName?: string }>
  emailAddresses?: Array<{ value?: string }>
  phoneNumbers?: Array<{ value?: string }>
  photos?: Array<{ url?: string }>
  organizations?: Array<{ name?: string; title?: string }>
}

export function GoogleContactsApp({ className }: { className?: string }) {
  const auth = useAuth()
  const profile = auth.unlockedProfile
  const [contacts, setContacts] = useState<GooglePerson[]>([])
  const [nextPageToken, setNextPageToken] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [password, setPassword] = useState("")
  const [nodeIdsByContact, setNodeIdsByContact] = useState<Record<string, string>>({})
  const [status, setStatus] = useState<string | null>(null)

  const localGoogleIds = useMemo(() => new Set(
    profile?.contacts.filter((contact) => contact.source === "google" && contact.sourceId).map((contact) => contact.sourceId as string) ?? [],
  ), [profile?.contacts])

  const connect = useCallback(async () => {
    const res = await fetch("/api/google-drive/auth")
    const data = await res.json()
    if (data.authUrl) window.location.href = data.authUrl
    else setError(data.error || "Failed to start Google connection")
  }, [])

  const fetchContacts = useCallback(async (pageToken?: string) => {
    setLoading(true)
    setError(null)
    try {
      const url = new URL("/api/google-contacts/contacts", window.location.origin)
      if (pageToken) url.searchParams.set("pageToken", pageToken)
      const res = await fetch(url)
      const data = await res.json()
      if (res.status === 401) {
        setError("Google session is missing or needs the Contacts scope. Reconnect Google.")
        return
      }
      if (!res.ok) throw new Error(data.error || "Failed to fetch contacts")
      setContacts((current) => pageToken ? [...current, ...(data.connections ?? [])] : (data.connections ?? []))
      setNextPageToken(data.nextPageToken ?? null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const disconnect = useCallback(() => {
    void fetch("/api/google-drive/session", { method: "DELETE" })
    setContacts([])
    setNextPageToken(null)
    setError("Google session disconnected. Reconnect Google to load contacts.")
  }, [])

  useEffect(() => {
    void fetchContacts()
  }, [fetchContacts])

  function personName(person: GooglePerson) {
    return person.names?.[0]?.displayName || person.emailAddresses?.[0]?.value || "Unnamed contact"
  }

  function personEmail(person: GooglePerson) {
    return person.emailAddresses?.[0]?.value ?? ""
  }

  function personPhone(person: GooglePerson) {
    return person.phoneNumbers?.[0]?.value ?? ""
  }

  async function importPerson(person: GooglePerson) {
    if (!password) {
      setStatus("Enter the profile password before importing.")
      return
    }
    const ok = await auth.saveContact({
      label: personName(person),
      email: personEmail(person),
      phone: personPhone(person),
      photoUrl: person.photos?.[0]?.url,
      source: "google",
      sourceId: person.resourceName,
      routeHint: "address-book",
      knownNodeIds: nodeIdsByContact[person.resourceName]?.split(/\r?\n|,/).map((item) => item.trim()).filter(Boolean) ?? [],
      password,
    })
    setStatus(ok ? `Imported ${personName(person)} into local profile.` : "Import failed.")
  }

  async function importAllVisible() {
    if (!password) {
      setStatus("Enter the profile password before importing.")
      return
    }
    let count = 0
    for (const person of contacts) {
      const ok = await auth.saveContact({
        label: personName(person),
        email: personEmail(person),
        phone: personPhone(person),
        photoUrl: person.photos?.[0]?.url,
        source: "google",
        sourceId: person.resourceName,
        routeHint: "address-book",
        knownNodeIds: nodeIdsByContact[person.resourceName]?.split(/\r?\n|,/).map((item) => item.trim()).filter(Boolean) ?? [],
        password,
      })
      if (ok) count += 1
    }
    setStatus(`Imported ${count} visible contact${count === 1 ? "" : "s"} into local profile.`)
  }

  return (
    <div className={cn("flex h-full flex-col bg-background", className)}>
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <UsersRound className="h-4 w-4 text-primary" />
          <div>
            <div className="text-sm font-semibold text-foreground">Google Contacts</div>
            <div className="text-[11px] text-muted-foreground">Import contacts into your local profile</div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <input
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            type="password"
            placeholder="Profile password"
            className="hidden h-8 w-36 rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-primary md:block"
          />
          <button onClick={importAllVisible} disabled={!contacts.length || !password || auth.isLoading} className="inline-flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-40" title="Import visible contacts into profile">
            <Download className="h-3.5 w-3.5" />
            Import visible
          </button>
          <button onClick={() => fetchContacts()} disabled={loading} className="rounded p-1.5 text-muted-foreground hover:bg-secondary hover:text-foreground" title="Refresh contacts">
            <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
          </button>
          <button onClick={disconnect} className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground" title="Disconnect Google session">
            <LogOut className="h-3.5 w-3.5" />
            Disconnect
          </button>
        </div>
      </div>
      <div className="border-b border-border px-4 py-2 text-[11px] text-muted-foreground">Imported contacts are saved into your encrypted profile, including any known node IDs you add here.</div>
      <div className="border-b border-border p-3 md:hidden">
        <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" placeholder="Profile password" className="h-8 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none focus:border-primary" />
      </div>
      {status && <div className="border-b border-border px-4 py-2 text-[11px] text-primary">{status}</div>}
      {error && (
        <div className="m-3 rounded-md border border-border bg-card p-3">
          <div className="mb-2 flex items-center gap-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>
          <button onClick={connect} className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-secondary">
            <ExternalLink className="h-3.5 w-3.5" /> Reconnect Google
          </button>
        </div>
      )}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {contacts.length === 0 && !loading ? <p className="p-6 text-center text-xs text-muted-foreground">No contacts loaded</p> : null}
        <div className="grid gap-1.5">
          {contacts.map((person) => {
            const name = personName(person)
            const email = personEmail(person)
            const phone = personPhone(person)
            const photo = person.photos?.[0]?.url
            const imported = localGoogleIds.has(person.resourceName)
            return (
              <div key={person.resourceName} className="grid gap-2 rounded-md border border-border bg-card px-2.5 py-2 md:grid-cols-[minmax(0,1fr)_220px_auto] md:items-center">
                <div className="flex min-w-0 items-center gap-2">
                  <div className="flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-md bg-secondary">
                    {photo ? (
                      <span className="h-full w-full bg-cover bg-center" style={{ backgroundImage: `url(${photo})` }} />
                    ) : (
                      <UserRound className="h-4 w-4 text-muted-foreground" />
                    )}
                  </div>
                  <div className="min-w-0">
                    <div className="truncate text-xs font-medium text-foreground">{name}</div>
                    <div className="flex min-w-0 gap-3 text-[10px] text-muted-foreground">
                      {email && <span className="truncate inline-flex items-center gap-1"><Mail className="h-3 w-3" />{email}</span>}
                      {phone && <span className="truncate inline-flex items-center gap-1"><Phone className="h-3 w-3" />{phone}</span>}
                    </div>
                  </div>
                </div>
                <textarea
                  value={nodeIdsByContact[person.resourceName] ?? ""}
                  onChange={(event) => setNodeIdsByContact((current) => ({ ...current, [person.resourceName]: event.target.value }))}
                  placeholder="Known node IDs"
                  className="min-h-8 rounded-md border border-border bg-background px-2 py-1 font-mono text-[10px] text-foreground outline-none focus:border-primary md:h-8 md:min-h-0"
                />
                <div className="flex items-center justify-end gap-2">
                  {imported && <span className="text-[10px] text-primary">local</span>}
                  <button onClick={() => importPerson(person)} disabled={!password || auth.isLoading} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border px-2 text-xs text-foreground hover:bg-secondary disabled:opacity-40">
                    <Save className="h-3.5 w-3.5" />
                    {imported ? "Update" : "Import"}
                  </button>
                </div>
              </div>
            )
          })}
        </div>
        {nextPageToken && <button onClick={() => fetchContacts(nextPageToken)} disabled={loading} className="mt-3 w-full rounded-md border border-border py-2 text-center text-xs text-primary hover:bg-secondary disabled:opacity-50">Load more</button>}
      </div>
    </div>
  )
}
