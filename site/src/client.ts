const buildMetaEl = document.querySelector<HTMLElement>('[data-build-meta]')
if (buildMetaEl) {
  buildMetaEl.title = `Rendered client-side at ${new Date().toISOString()}`
}

const versionSelect = document.querySelector<HTMLSelectElement>('#version-select')
if (versionSelect) {
  versionSelect.addEventListener('change', () => {
    const v = versionSelect.value
    if (!v) return
    window.location.href = `/docs/${v}/`
  })
}

