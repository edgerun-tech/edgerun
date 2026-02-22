// SPDX-License-Identifier: Apache-2.0
import { JSX } from 'solid-js'
import { Nav } from '../nav'
import { Footer } from '../footer'

export function PageShell(props: { children: JSX.Element }) {
  return (
    <div class="flex min-h-screen flex-col">
      <Nav />
      <main class="flex-1">{props.children}</main>
      <Footer />
    </div>
  )
}
