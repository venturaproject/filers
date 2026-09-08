/** Opciones de filas por página disponibles en todas las tablas paginadas */
export const PAGINATION_OPTIONS = [10, 20, 50, 100] as const

/** Breakpoint en píxeles por debajo del cual se considera mobile */
export const MOBILE_BREAKPOINT = 768

/** Tiempo de debounce en ms para campos de búsqueda que disparan navegación */
export const SEARCH_DEBOUNCE_MS = 300

/** TTL en ms para cachés de lookups (phone lookup, etc.) */
export const LOOKUP_CACHE_STALE_MS = 5 * 60 * 1000
