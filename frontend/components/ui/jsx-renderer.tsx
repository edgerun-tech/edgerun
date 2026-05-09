"use client"

import JsxParser from "react-jsx-parser"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Input } from "@/components/ui/input"

export type JsxRendererProps = {
  jsx: string
  className?: string
}

const components = {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Input,
}

export function JsxRenderer({ jsx, className }: JsxRendererProps) {
  return (
    <JsxParser
      allowUnknownElements={false}
      components={components}
      jsx={jsx}
      className={className}
      renderError={({ error }) => (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 p-2 text-[10px] text-destructive">
          {error}
        </div>
      )}
    />
  )
}
