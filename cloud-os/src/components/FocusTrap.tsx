import { FocusTrap } from '@ark-ui/solid/focus-trap'
import { createSignal } from 'solid-js'

export const Basic = () => {
  const [trapped, setTrapped] = createSignal(false)
  return (
    <>
      <button onClick={() => setTrapped(true)}>Start Trap</button>
      <FocusTrap returnFocusOnDeactivate={false} disabled={!trapped()}>
        <div class="flex flex-col gap-4 py-4">
          <input type="text" placeholder="input" />
          <textarea placeholder="textarea" />
          <button onClick={() => setTrapped(false)}>End Trap</button>
        </div>
      </FocusTrap>
    </>
  )
}