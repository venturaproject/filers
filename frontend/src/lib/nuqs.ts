import { router } from '@/lib/router-singleton'
import { useLocation } from 'react-router-dom'
import {
  unstable_AdapterOptions as AdapterOptions,
  unstable_createAdapterProvider as createAdapterProvider,
  renderQueryString,
} from 'nuqs/adapters/custom'

function useNuqsInertiaAdapter() {
  const location = useLocation()
  const searchParams = new URLSearchParams(location.search)

  const updateUrl = (search: URLSearchParams, options: AdapterOptions) => {
    router.visit(`${window.location.pathname}${renderQueryString(search)}`, {
      replace: options.history === 'replace',
      preserveScroll: !options.scroll,
      preserveState: true,
    })
  }

  return {
    searchParams,
    updateUrl,
  }
}

export const NuqsAdapter = createAdapterProvider(useNuqsInertiaAdapter)
