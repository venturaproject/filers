import { createColumnHelper } from '@tanstack/react-table'
import { Badge } from '@/components/ui/badge'
import type { JobKind, JobOrigin, JobSummary, Operation } from '@/services/files-api'
import { JobStatusBadge, STATUS_LABEL } from './status-badge'

export { STATUS_LABEL }

export const OPERATION_LABEL: Record<Operation, string> = {
  parse: 'Parsear',
  profile: 'Perfilar',
  validate: 'Validar',
  convert: 'Convertir',
  transform: 'Transformar',
  pipeline: 'Pipeline',
  batch: 'Lote',
}

export const KIND_LABEL: Record<JobKind, string> = {
  sync: 'Síncrono',
  batch: 'Batch',
}

export const ORIGIN_LABEL: Record<JobOrigin, string> = {
  api_key: 'API key',
  oauth_client: 'OAuth',
  admin: 'Panel',
  service: 'Servicio',
}

export function formatDuration(ms: number | null): string {
  if (ms == null) return '—'
  if (ms < 1000) return `${ms} ms`
  return `${(ms / 1000).toFixed(1)} s`
}

const col = createColumnHelper<JobSummary>()

export function buildJobsColumns() {
  return [
    col.accessor('label', {
      id: 'label',
      header: () => 'Trabajo',
      cell: (info) => (
        <div className="min-w-0">
          <div className="truncate font-medium">
            {info.getValue() ?? `${info.row.original.files_total} archivo(s)`}
          </div>
          <div className="font-mono text-xs text-muted-foreground">
            {info.row.original.id.slice(0, 8)}
          </div>
        </div>
      ),
    }),
    col.accessor('status', {
      id: 'status',
      header: () => 'Estado',
      cell: (info) => <JobStatusBadge status={info.getValue()} />,
    }),
    col.accessor('kind', {
      id: 'kind',
      header: () => 'Modo',
      cell: (info) => (
        <Badge variant="secondary" className="font-normal">
          {KIND_LABEL[info.getValue()]}
        </Badge>
      ),
    }),
    col.accessor('operation', {
      id: 'operation',
      header: () => 'Operación',
      cell: (info) => (
        <Badge variant="outline" className="font-normal">
          {OPERATION_LABEL[info.getValue()] ?? info.getValue()}
        </Badge>
      ),
    }),
    col.accessor('origin', {
      id: 'origin',
      header: () => 'Origen',
      cell: (info) => (
        <div className="text-sm">
          <Badge variant="outline" className="font-normal">
            {ORIGIN_LABEL[info.getValue()]}
          </Badge>
          {info.row.original.actor && (
            <div className="mt-0.5 truncate text-xs text-muted-foreground">
              {info.row.original.actor}
            </div>
          )}
        </div>
      ),
    }),
    col.display({
      id: 'files',
      header: () => 'Archivos',
      cell: ({ row }) => {
        const { files_processed, files_failed, files_total } = row.original
        return (
          <span className="text-sm">
            {files_processed}
            {files_failed > 0 && <span className="text-destructive"> · {files_failed} err</span>}
            <span className="text-muted-foreground"> / {files_total}</span>
          </span>
        )
      },
    }),
    col.accessor('total_rows', {
      id: 'total_rows',
      header: () => 'Filas',
      cell: (info) => (
        <span className="tabular-nums">{info.getValue().toLocaleString('es-ES')}</span>
      ),
    }),
    col.accessor('duration_ms', {
      id: 'duration_ms',
      header: () => 'Duración',
      cell: (info) => <span className="tabular-nums">{formatDuration(info.getValue())}</span>,
    }),
    col.accessor('created_at', {
      id: 'created_at',
      header: () => 'Creado',
      cell: (info) => (
        <span className="text-xs text-muted-foreground">
          {new Date(info.getValue()).toLocaleString('es-ES')}
        </span>
      ),
    }),
    col.accessor('completed_at', {
      id: 'completed_at',
      header: () => 'Finalizado',
      cell: (info) => (
        <span className="text-xs text-muted-foreground">
          {info.getValue() ? new Date(info.getValue() as string).toLocaleString('es-ES') : '—'}
        </span>
      ),
    }),
  ]
}

export const jobColumnLabels: Record<string, string> = {
  label: 'Trabajo',
  status: 'Estado',
  kind: 'Modo',
  operation: 'Operación',
  origin: 'Origen',
  files: 'Archivos',
  total_rows: 'Filas',
  duration_ms: 'Duración',
  created_at: 'Creado',
  completed_at: 'Finalizado',
}
