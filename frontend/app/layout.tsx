import type { Metadata, Viewport } from 'next'
import { Geist, Geist_Mono } from 'next/font/google'
import './globals.css'
import { PlatformProvider } from '@/platform/ui/PlatformProvider'
import { TooltipProvider } from '@radix-ui/react-tooltip'

const geist = Geist({ subsets: ["latin"], variable: "--font-geist-sans" });
const geistMono = Geist_Mono({ subsets: ["latin"], variable: "--font-geist-mono" });

export const metadata: Metadata = {
  title: 'Edgerun — Distributed Runtime',
  description: 'Edgerun: a distributed WASI app runtime secured by biometrics — your OS at the edge.',
  icons: {
    icon: [
      {
        url: '/icon-light-32x32.png',
        media: '(prefers-color-scheme: light)',
      },
      {
        url: '/icon-dark-32x32.png',
        media: '(prefers-color-scheme: dark)',
      },
      {
        url: '/icon.svg',
        type: 'image/svg+xml',
      },
    ],
    apple: '/apple-icon.png',
  },
}

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" className="bg-background">
      <body className={`${geist.variable} ${geistMono.variable} font-sans antialiased overflow-hidden`}>
        <TooltipProvider>
          <PlatformProvider>
            {children}
          </PlatformProvider>
        </TooltipProvider>
      </body>
    </html>
  )
}
