type NavigateFn = (to: string, options?: { replace?: boolean; state?: any }) => void

let _navigate: NavigateFn | null = null

export function setNavigate(fn: NavigateFn) {
  _navigate = fn
}

function buildUrl(url: string, params?: Record<string, any>): string {
  if (!params || Object.keys(params).length === 0) return url
  const entries = Object.entries(params).filter(
    ([, v]) => v !== undefined && v !== null && v !== ''
  )
  if (!entries.length) return url
  return url + '?' + new URLSearchParams(entries.map(([k, v]) => [k, String(v)])).toString()
}

type RouterRequestOptions = {
  onSuccess?: (page?: any) => void
  onError?: (errors?: any) => void
  onFinish?: () => void
  onStart?: () => void
  preserveScroll?: boolean
  preserveState?: boolean
  replace?: boolean
}

async function httpRequest(
  method: 'post' | 'put' | 'patch' | 'delete',
  url: string,
  data?: Record<string, any>,
  options?: RouterRequestOptions,
) {
  try {
    // Dynamic import avoids potential circular dependency at module init time.
    const { axios } = await import('./axios')
    const response = await axios[method](url, data ?? {})
    options?.onSuccess?.(response.data)
  } catch (err: any) {
    const errors = err.response?.data?.errors ?? err.response?.data ?? {}
    options?.onError?.(errors)
  } finally {
    options?.onFinish?.()
  }
}

export const router = {
  get(url: string, params?: Record<string, any>, options?: { replace?: boolean }) {
    if (!_navigate) { window.location.href = buildUrl(url, params); return }
    _navigate(buildUrl(url, params), { replace: options?.replace })
  },

  visit(url: string, options?: { replace?: boolean; preserveScroll?: boolean; preserveState?: boolean }) {
    if (!_navigate) { window.location.href = url; return }
    _navigate(url, { replace: options?.replace })
  },

  reload() {
    window.location.reload()
  },

  post(url: string, data?: Record<string, any>, options?: RouterRequestOptions) {
    return httpRequest('post', url, data, options)
  },

  put(url: string, data?: Record<string, any>, options?: RouterRequestOptions) {
    return httpRequest('put', url, data, options)
  },

  patch(url: string, data?: Record<string, any>, options?: RouterRequestOptions) {
    return httpRequest('patch', url, data, options)
  },

  delete(url: string, data?: Record<string, any>, options?: RouterRequestOptions) {
    return httpRequest('delete', url, data, options)
  },
}
