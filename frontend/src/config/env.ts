// Nombre visible de la app (título del documento, footer, login).
//
// `VITE_APP_NAME` es solo el FALLBACK que se hornea en build. El valor real llega
// en runtime desde el backend (`GET /api/v1/config` → APP_NAME), y `app.tsx` lo
// aplica con `applyRuntimeConfig()` antes del primer render. Es un `let` con
// binding vivo: al reasignarlo aquí, todos los `import { APP_NAME }` ven el valor
// nuevo. Cambiarlo no requiere rebuild del frontend, solo reiniciar el backend.
export let APP_NAME: string = import.meta.env.VITE_APP_NAME || "Filers"

export interface RuntimeConfig {
  app_name?: string
}

export function applyRuntimeConfig(cfg: RuntimeConfig): void {
  if (cfg.app_name) APP_NAME = cfg.app_name
}

export const APP_URL = import.meta.env.VITE_APP_URL ?? ""

export const ASSET_URL = import.meta.env.VITE_ASSET_URL ?? ""

export const PUBLIC_API_URL = import.meta.env.VITE_PUBLIC_API_URL ?? ""

export const STORAGE_PREFIX = import.meta.env.VITE_STORAGE_PREFIX ?? "__ans_"

// Roles that bypass all permission checks. Set VITE_FULL_ACCESS_ROLES in .env
// to override (comma-separated). Defaults to "admin".
export const FULL_ACCESS_ROLES: string[] = (import.meta.env.VITE_FULL_ACCESS_ROLES ?? 'admin')
  .split(',')
  .map((r: string) => r.trim())
  .filter(Boolean)

export const env = {
  get APP_NAME() {
    return APP_NAME
  },
  APP_URL,
  ASSET_URL,
  PUBLIC_API_URL,
  STORAGE_PREFIX,
  FULL_ACCESS_ROLES,
}
