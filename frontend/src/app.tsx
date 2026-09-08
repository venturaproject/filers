import './index.css'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router-dom'
import { Providers } from '@/providers'
import { ErrorBoundary } from '@/components/error-boundary'
import { createAppRouter } from '@/lib/router'
import { privateRoutes, guestRoutes } from '@/app-routes'
import { queryClient } from '@/lib/react-query'
import { APP_NAME, applyRuntimeConfig } from '@/config/env'
import { fetchPublicConfig } from '@/services/config-api'

const router = createAppRouter({ privateRoutes, guestRoutes })

async function bootstrap() {
  // Runtime config from the backend (GET /api/v1/config). On failure we keep the
  // build-time VITE_APP_NAME fallback.
  try {
    const cfg = await fetchPublicConfig()
    applyRuntimeConfig(cfg)
    queryClient.setQueryData(['config'], cfg)
  } catch {
    /* offline / backend down — fallback stays */
  }
  document.title = APP_NAME

  createRoot(document.getElementById('app')!).render(
    <StrictMode>
      <Providers>
        <ErrorBoundary>
          <RouterProvider router={router} />
        </ErrorBoundary>
      </Providers>
    </StrictMode>,
  )
}

void bootstrap()
