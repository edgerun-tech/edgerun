# CDP Toolkit

Dependency-free JavaScript tools for automating a local Chrome/Brave/Chromium instance through the Chrome DevTools Protocol.

Start the browser with a debugging port:

```sh
brave --remote-debugging-port=9222
```

Common commands:

```sh
npm run cdp:list
npm run cdp -- version
npm run cdp -- focus 'placeholder=Ask anything' --target chatgpt.com
npm run cdp -- type 'hello from CDP' --target chatgpt.com
npm run cdp -- click 'text=Submit'
npm run cdp -- eval 'location.href'
npm run cdp -- observe --duration 3000 --target dash.edgerun.tech
npm run cdp -- storage --target dash.edgerun.tech
npm run cdp -- cookies --target dash.edgerun.tech
npm run cdp -- screenshot tmp/page.png --target dash.edgerun.tech
```

Locator formats:

- CSS selector: `button[type=submit]`
- Text: `text=Save`
- Placeholder or accessible input text: `placeholder=Ask anything`
- Role: `role=textbox`
- Test id: `testid=submit-button`

The core modules are reusable from scripts:

```js
import { CdpPage } from "./tools/cdp/page.mjs"

const page = await CdpPage.connect({ target: "dash.edgerun.tech" })
await page.bringToFront()
await page.click("text=Deploy")
await page.screenshot("tmp/deploy.png")
page.close()
```
