// Singleton que guarda la función navigate de React Router
// para usarla fuera de componentes (en router compat, interceptores, etc.)
let _navigate: ((to: string, options?: { replace?: boolean }) => void) | null = null

export function setNavigate(fn: (to: string, options?: { replace?: boolean }) => void) {
  _navigate = fn
}

export function navigateTo(to: string, options?: { replace?: boolean }) {
  if (_navigate) {
    _navigate(to, options)
  } else {
    window.location.href = to
  }
}
