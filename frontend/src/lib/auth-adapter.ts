import axios from 'axios'
import type { AuthUser } from './auth'

// ── Types ─────────────────────────────────────────────────────────────────────

export interface SessionData {
  user: AuthUser
  permissions: string[]
  roles: string[]
}

export interface AuthAdapter {
  /** Called before protected requests to attach headers (e.g. CSRF token). */
  getRequestHeaders(): Record<string, string>
  /** Perform login; returns session data on success, throws on failure. */
  login(credentials: Record<string, unknown>): Promise<SessionData>
  /** Perform logout. */
  logout(): Promise<void>
  /** Fetch the current session from the backend. Returns null if unauthenticated. */
  fetchSession(): Promise<SessionData | null>
  /** Attempt to refresh the access token. Returns true if successful. */
  refresh(): Promise<boolean>
  /** Called when the session cannot be recovered (e.g. redirect to /login). */
  onUnauthorized(): void
}

// ── Default implementation: Django + HttpOnly cookies + CSRF ──────────────────

interface DjangoAdapterConfig {
  loginUrl: string
  logoutUrl: string
  meUrl: string
  refreshUrl: string
  /** If set, a GET to this URL is made before login to seed the CSRF cookie. */
  csrfUrl?: string
  /** Redirect path after an unrecoverable 401. Defaults to '/login'. */
  unauthorizedRedirect?: string
}

// Bare axios instance used only by the adapter — no interceptors, no cycles.
const _http = axios.create({ withCredentials: true, headers: { 'Content-Type': 'application/json' } })

// Attach CSRF token to mutating requests so Django accepts logout/refresh POSTs.
_http.interceptors.request.use((config) => {
  if (config.method && ['post', 'put', 'patch', 'delete'].includes(config.method)) {
    const token = document.cookie.split('; ').find((r) => r.startsWith('csrftoken='))?.split('=')[1]
    if (token) config.headers['X-CSRFToken'] = token
  }
  return config
})

export class DjangoHttpOnlyAdapter implements AuthAdapter {
  private readonly cfg: DjangoAdapterConfig

  constructor(cfg: DjangoAdapterConfig) {
    this.cfg = cfg
  }

  getRequestHeaders(): Record<string, string> {
    const token = document.cookie
      .split('; ')
      .find((row) => row.startsWith('csrftoken='))
      ?.split('=')[1]
    return token ? { 'X-CSRFToken': token } : {}
  }

  async login(credentials: Record<string, unknown>): Promise<SessionData> {
    if (this.cfg.csrfUrl) await _http.get(this.cfg.csrfUrl)
    const { data } = await _http.post(this.cfg.loginUrl, credentials)
    const user: AuthUser = data.user ?? data
    return { user, permissions: user.permissions ?? [], roles: user.roles ?? [] }
  }

  async logout(): Promise<void> {
    await _http.post(this.cfg.logoutUrl).catch(() => {})
  }

  async fetchSession(): Promise<SessionData | null> {
    try {
      const { data } = await _http.get(this.cfg.meUrl)
      const user: AuthUser = data.user ?? data
      return { user, permissions: user.permissions ?? [], roles: user.roles ?? [] }
    } catch {
      return null
    }
  }

  async refresh(): Promise<boolean> {
    try {
      await _http.post(this.cfg.refreshUrl)
      return true
    } catch {
      return false
    }
  }

  onUnauthorized(): void {
    window.location.href = this.cfg.unauthorizedRedirect ?? '/login'
  }
}

// ── Singleton ─────────────────────────────────────────────────────────────────
// Replace with setAuthAdapter() in your app entry point to swap backends.

let _adapter: AuthAdapter = new DjangoHttpOnlyAdapter({
  loginUrl:    '/api/v1/auth/login',
  logoutUrl:   '/api/v1/auth/logout',
  meUrl:       '/api/v1/auth/me',
  refreshUrl:  '/api/v1/auth/refresh',
  csrfUrl:     '/api/v1/csrf/',
})

export const getAuthAdapter = (): AuthAdapter => _adapter

/** Call this once (e.g. in main.tsx) to swap the default adapter. */
export const setAuthAdapter = (adapter: AuthAdapter): void => {
  _adapter = adapter
}
