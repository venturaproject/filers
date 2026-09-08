/**
 * Submits a hidden HTML form for bulk actions that still need full form POST.
 * This stays backend-agnostic at the transport level; only the target path is app-specific.
 */
export function submitBulkActionForm(
  actionPath: string,
  selectedIds: Set<string | number>,
  selectAllRecords: boolean,
  filters: Record<string, string | undefined>
) {
  const form = document.createElement('form')
  form.method = 'POST'
  form.action = actionPath

  const csrf =
    document.querySelector<HTMLMetaElement>('meta[name="csrf-token"]')?.content ?? ''
  const csrfInput = document.createElement('input')
  csrfInput.type = 'hidden'
  csrfInput.name = '_token'
  csrfInput.value = csrf
  form.appendChild(csrfInput)

  if (selectAllRecords) {
    const allInput = document.createElement('input')
    allInput.type = 'hidden'
    allInput.name = 'select_all'
    allInput.value = '1'
    form.appendChild(allInput)

    Object.entries(filters).forEach(([key, value]) => {
      if (value) {
        const input = document.createElement('input')
        input.type = 'hidden'
        input.name = `filters[${key}]`
        input.value = String(value)
        form.appendChild(input)
      }
    })
  } else {
    Array.from(selectedIds).forEach((id) => {
      const input = document.createElement('input')
      input.type = 'hidden'
      input.name = 'ids[]'
      input.value = String(id)
      form.appendChild(input)
    })
  }

  document.body.appendChild(form)
  form.submit()
  document.body.removeChild(form)
}
