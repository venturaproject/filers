type RouteParams = Record<string, any> | string | number | null | undefined
type RouteValue = string | ((params: Record<string, any>) => string)

export const appRouteMap: Record<string, RouteValue> = {
  login: '/login',
  logout: '/logout',
  'password.request': '/forgot-password',
  'password.reset': (p) => `/reset-password/${p.token}`,
  'verification.notice': '/verify-email',
  'password.confirm': '/confirm-password',

  dashboard: '/admin',

  'profile.edit': '/admin/settings',
  'profile.update': '/api/v1/auth/me',
  'profile.destroy': '/api/v1/auth/me',
  'profile.avatar.update': '/api/v1/auth/me/avatar',
  'dashboard.settings.profile': '/admin/settings',
  'dashboard.settings.permissions': '/admin/settings/permissions',
  'dashboard.settings.appearance': '/admin/settings/appearance',
  'dashboard.settings.appearance.update': '/admin/settings/appearance',
  'dashboard.settings.notifications': '/admin/settings/notifications',
  'dashboard.settings.notifications.update': '/admin/settings/notifications',
  'dashboard.settings.display': '/admin/settings/display',
  'dashboard.settings.display.update': '/admin/settings/display',
  'dashboard.settings.account': '/admin/settings/account',
  'dashboard.settings.avatar.update': '/admin/settings/avatar',

  'admin.users.index': '/admin/users',
  'admin.users.create': '/admin/users/create',
  'admin.users.store': '/admin/users',
  'admin.users.show': (p) => `/admin/users/${p.user ?? p.id}`,
  'admin.users.edit': (p) => `/admin/users/${p.user ?? p.id}/edit`,
  'admin.users.update': (p) => `/admin/users/${p.user ?? p.id}`,
  'admin.users.destroy': (p) => `/admin/users/${p.user ?? p.id}`,
  'admin.users.bulk-export': '/admin/users/bulk-export',

  'admin.roles.index': '/admin/roles',
  'admin.roles.create': '/admin/roles/create',
  'admin.roles.store': '/admin/roles',
  'admin.roles.edit': (p) => `/admin/roles/${p.role ?? p.id}/edit`,
  'admin.roles.update': (p) => `/admin/roles/${p.role ?? p.id}`,
  'admin.roles.destroy': (p) => `/admin/roles/${p.role ?? p.id}`,

  'admin.permissions.index': '/admin/permissions',
  'admin.permissions.create': '/admin/permissions/create',
  'admin.permissions.store': '/admin/permissions',
  'admin.permissions.edit': (p) => `/admin/permissions/${p.permission ?? p.id}/edit`,
  'admin.permissions.update': (p) => `/admin/permissions/${p.permission ?? p.id}`,
  'admin.permissions.destroy': (p) => `/admin/permissions/${p.permission ?? p.id}`,

  'admin.files.process': '/admin/process',
  'admin.files.jobs': '/admin/jobs',
  'admin.files.job': (p) => `/admin/jobs/${p.id}`,
}

function normalizeParams(params?: RouteParams): Record<string, any> {
  if (params === null || params === undefined) return {}
  if (typeof params === 'object' && !Array.isArray(params)) return params
  return { id: params }
}

function toQueryString(params: Record<string, any>): string {
  const entries = Object.entries(params).filter(
    ([, v]) => v !== undefined && v !== null && v !== '',
  )
  if (!entries.length) return ''
  return '?' + new URLSearchParams(entries.map(([k, v]) => [k, String(v)])).toString()
}

export function pathFor(name?: string, params?: RouteParams): string {
  if (!name) return window.location.pathname
  const entry = appRouteMap[name]
  if (!entry) {
    console.warn(`[pathFor] Unknown route name: "${name}"`)
    return '#'
  }
  if (typeof entry === 'function') return entry(normalizeParams(params))
  // Static path: an object of params becomes a query string (e.g. list filters).
  const isObject =
    params !== null && params !== undefined && typeof params === 'object' && !Array.isArray(params)
  return isObject ? entry + toQueryString(params as Record<string, any>) : entry
}

pathFor.current = (name: string): boolean => {
  const target = pathFor(name)
  return window.location.pathname === target || window.location.pathname.startsWith(target + '/')
}

export default pathFor
