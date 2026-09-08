/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_APP_NAME?: string
  readonly VITE_APP_URL?: string
  readonly VITE_ASSET_URL?: string
  readonly VITE_PUBLIC_API_URL?: string
  readonly VITE_BACKEND_URL?: string
  readonly VITE_STORAGE_PREFIX?: string
  readonly VITE_FULL_ACCESS_ROLES?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
