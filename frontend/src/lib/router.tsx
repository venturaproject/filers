import { useEffect } from 'react'
import {
  createBrowserRouter,
  Navigate,
  Outlet,
  type RouteObject,
  useNavigate,
} from 'react-router-dom'
import { Loader2 } from 'lucide-react'
import { NavigationLoader } from '@/components/navigation-loader'
import { useAuthStore } from '@/lib/auth'
import { setNavigate } from '@/lib/router-singleton'
import NotFoundError from '@/pages/errors/not-found-error'

// ── Types ─────────────────────────────────────────────────────────────────────

export interface RouterConfig {
  /** Routes that require an authenticated session. */
  privateRoutes: RouteObject[]
  /** Routes accessible only when NOT authenticated. */
  guestRoutes?: RouteObject[]
  /** Where to redirect the root path '/' and after login. Defaults to '/admin'. */
  homePath?: string
  /** Where to send unauthenticated users. Defaults to '/login'. */
  loginPath?: string
}

// ── Internal layout pieces ────────────────────────────────────────────────────

function RootLayout() {
  return (
    <>
      <NavigationLoader />
      <Outlet />
    </>
  )
}

function NavigateSetter() {
  const navigate = useNavigate()
  useEffect(() => { setNavigate(navigate) }, [navigate])
  return null
}

function HydrationSpinner() {
  return (
    <div className="flex h-screen items-center justify-center">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
    </div>
  )
}

// ── Guard factories ───────────────────────────────────────────────────────────

function makePrivateGuard(loginPath: string) {
  return function PrivateGuard() {
    const { initialized, isAuthenticated, hydrate } = useAuthStore()
    useEffect(() => { hydrate() }, [])
    if (!initialized) return <HydrationSpinner />
    if (!isAuthenticated()) return <Navigate to={loginPath} replace />
    return <><NavigateSetter /><Outlet /></>
  }
}

function makeGuestGuard(homePath: string) {
  return function GuestGuard() {
    const { initialized, isAuthenticated, hydrate } = useAuthStore()
    useEffect(() => { hydrate() }, [])
    if (!initialized) return <HydrationSpinner />
    if (isAuthenticated()) return <Navigate to={homePath} replace />
    return <><NavigateSetter /><Outlet /></>
  }
}

// ── Factory ───────────────────────────────────────────────────────────────────

/**
 * Build a browser router from a route config.
 *
 * Usage:
 *   const router = createAppRouter({ privateRoutes, guestRoutes })
 *
 * To change the auth paths or home redirect, pass homePath / loginPath.
 */
export function createAppRouter({
  privateRoutes,
  guestRoutes = [],
  homePath = '/admin',
  loginPath = '/login',
}: RouterConfig) {
  const PrivateGuard = makePrivateGuard(loginPath)
  const GuestGuard = makeGuestGuard(homePath)

  return createBrowserRouter([
    {
      element: <RootLayout />,
      children: [
        {
          element: <GuestGuard />,
          children: guestRoutes,
        },
        {
          element: <PrivateGuard />,
          children: privateRoutes,
        },
        { path: '/', element: <Navigate to={homePath} replace /> },
        { path: '*', element: <NotFoundError /> },
      ],
    },
  ])
}
