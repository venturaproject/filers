import { create } from 'zustand'
import { getAuthAdapter } from './auth-adapter'

export interface AuthUser {
  id: number
  name: string
  username?: string | null
  email: string
  role?: string
  avatar: string | null
  permissions?: string[]
  roles?: string[]
}

interface AuthState {
  user: AuthUser | null
  permissions: string[]
  roles: string[]
  /** True once fetchSession has completed at least once (success or failure). */
  initialized: boolean

  setAuth: (user: AuthUser, permissions?: string[], roles?: string[]) => void
  logout: () => void
  isAuthenticated: () => boolean
  /** Hydrate session from the backend. Safe to call multiple times — only runs once. */
  hydrate: () => Promise<void>
  /** Like hydrate() but always re-fetches regardless of initialized state. */
  rehydrate: () => Promise<void>
}

// BroadcastChannel syncs auth events across tabs; does NOT fire in the sending tab.
const authChannel = typeof window !== 'undefined' ? new BroadcastChannel('auth') : null

export const useAuthStore = create<AuthState>()((set, get) => {
  const _fetchAndSet = async () => {
    const session = await getAuthAdapter().fetchSession()
    if (session) {
      set({ user: session.user, permissions: session.permissions, roles: session.roles, initialized: true })
    } else {
      set({ user: null, permissions: [], roles: [], initialized: true })
    }
  }

  // When another tab logs out, clear this tab's state and redirect.
  authChannel?.addEventListener('message', (e) => {
    if (e.data?.type === 'LOGOUT') {
      set({ user: null, permissions: [], roles: [], initialized: true })
      window.location.replace('/login')
    }
  })

  return {
    user: null,
    permissions: [],
    roles: [],
    initialized: false,

    setAuth: (user, permissions = [], roles = []) =>
      set({ user, permissions, roles, initialized: true }),

    logout: () => {
      authChannel?.postMessage({ type: 'LOGOUT' })
      set({ user: null, permissions: [], roles: [], initialized: true })
    },

    isAuthenticated: () => get().initialized && get().user !== null,

    hydrate: async () => {
      if (get().initialized) return
      await _fetchAndSet()
    },

    rehydrate: async () => {
      await _fetchAndSet()
    },
  }
})

// Re-validate the session when the tab comes back into focus, throttled to once per minute.
// If the session has expired, PrivateGuard will redirect to /login on the next render.
let _lastRehydrateAt = 0

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (
      document.visibilityState === 'visible' &&
      useAuthStore.getState().isAuthenticated() &&
      Date.now() - _lastRehydrateAt > 60_000
    ) {
      _lastRehydrateAt = Date.now()
      useAuthStore.getState().rehydrate()
    }
  })
}
