"use client"

import { useState } from "react"
import { Users, MessageSquare, Phone } from "lucide-react"
import { cn } from "@/lib/utils"
import { ContactsApp } from "@/components/os/contacts-app"
import { DemoChatApp } from "@/components/os/chat-app"
import { CallingApp } from "@/components/os/calling-app"

type PeopleTab = "contacts" | "messages" | "calls"

const TABS: { id: PeopleTab; label: string; icon: React.ReactNode }[] = [
  { id: "contacts", label: "Contacts", icon: <Users className="h-3.5 w-3.5" /> },
  { id: "messages", label: "Messages", icon: <MessageSquare className="h-3.5 w-3.5" /> },
  { id: "calls", label: "Calls", icon: <Phone className="h-3.5 w-3.5" /> },
]

export function PeopleApp() {
  const [tab, setTab] = useState<PeopleTab>("contacts")

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <div className="flex items-center gap-1 border-b border-border bg-[var(--window-header)] px-2 py-2">
        {TABS.map((item) => (
          <button
            key={item.id}
            onClick={() => setTab(item.id)}
            className={cn(
              "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
              tab === item.id
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-secondary hover:text-foreground",
            )}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-hidden">
        {tab === "contacts" && (
          <ContactsApp
            onCall={() => setTab("calls")}
            onMessage={() => setTab("messages")}
          />
        )}
        {tab === "messages" && <DemoChatApp />}
        {tab === "calls" && <CallingApp />}
      </div>
    </div>
  )
}
