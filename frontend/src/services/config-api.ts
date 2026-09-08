import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'
import type { RuntimeConfig } from '@/config/env'

export type PublicConfig = Required<RuntimeConfig>

/** Public config the SPA loads once at boot (no auth). Falls back to build-time
 *  values on failure — see `applyRuntimeConfig` / `app.tsx`. */
export async function fetchPublicConfig(timeoutMs = 3000): Promise<PublicConfig> {
  const { data } = await axios.get<PublicConfig>(endpoints.config, { timeout: timeoutMs })
  return data
}
