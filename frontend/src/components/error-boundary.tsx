import { Component, type ErrorInfo, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

/**
 * ErrorBoundary genérico para capturar errores de renderizado en React.
 * Úsalo para envolver secciones críticas de la UI (editores rich text, tablas complejas, etc.)
 *
 * @example
 * <ErrorBoundary>
 *   <ComponenteComplejo />
 * </ErrorBoundary>
 *
 * @example con fallback personalizado
 * <ErrorBoundary fallback={<p>Este módulo no está disponible</p>}>
 *   <EditorTiptap />
 * </ErrorBoundary>
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // En producción conectar aquí con Sentry / Bugsnag / etc.
    console.error('[ErrorBoundary] Error capturado:', error, info.componentStack)
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback

      return (
        <div className="flex min-h-[200px] flex-col items-center justify-center gap-4 rounded-lg border border-destructive/30 bg-destructive/5 p-8 text-center">
          <p className="text-sm font-medium text-destructive">
            Se ha producido un error inesperado en este componente.
          </p>
          {import.meta.env.DEV && this.state.error && (
            <pre className="max-w-full overflow-auto rounded bg-muted p-3 text-left text-xs text-muted-foreground">
              {this.state.error.message}
            </pre>
          )}
          <Button variant="outline" size="sm" onClick={this.handleReset}>
            Reintentar
          </Button>
        </div>
      )
    }

    return this.props.children
  }
}
