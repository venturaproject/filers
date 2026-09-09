import { CheckCircle2, CircleDashed, Loader2, XCircle, type LucideIcon } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { JobStatus } from '@/services/files-api'

type StatusMeta = {
  label: string
  icon: LucideIcon
  /** tint applied on top of the `outline` badge variant */
  className: string
  spin?: boolean
}

export const JOB_STATUS_META: Record<JobStatus, StatusMeta> = {
  pending: {
    label: 'En cola',
    icon: CircleDashed,
    className: 'border-border bg-muted text-muted-foreground',
  },
  running: {
    label: 'Procesando',
    icon: Loader2,
    className: 'border-sky-500/25 bg-sky-500/10 text-sky-700 dark:text-sky-300',
    spin: true,
  },
  completed: {
    label: 'Completado',
    icon: CheckCircle2,
    className: 'border-emerald-500/25 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  },
  failed: {
    label: 'Fallido',
    icon: XCircle,
    className: 'border-destructive/25 bg-destructive/10 text-destructive',
  },
}

export const STATUS_LABEL: Record<JobStatus, string> = {
  pending: JOB_STATUS_META.pending.label,
  running: JOB_STATUS_META.running.label,
  completed: JOB_STATUS_META.completed.label,
  failed: JOB_STATUS_META.failed.label,
}

export function JobStatusBadge({
  status,
  className,
}: {
  status: JobStatus
  className?: string
}) {
  const meta = JOB_STATUS_META[status] ?? JOB_STATUS_META.pending
  const Icon = meta.icon
  return (
    <Badge variant="outline" className={cn('gap-1.5 font-medium', meta.className, className)}>
      <Icon className={cn('h-3.5 w-3.5', meta.spin && 'animate-spin')} aria-hidden />
      {meta.label}
    </Badge>
  )
}

/** Per-file / per-step result pill (ok vs error). */
export function OutcomeBadge({
  ok,
  okLabel = 'OK',
  errorLabel = 'Error',
  className,
}: {
  ok: boolean
  okLabel?: string
  errorLabel?: string
  className?: string
}) {
  return (
    <Badge
      variant="outline"
      className={cn(
        'gap-1.5 font-medium',
        ok
          ? 'border-emerald-500/25 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300'
          : 'border-destructive/25 bg-destructive/10 text-destructive',
        className,
      )}
    >
      {ok ? (
        <CheckCircle2 className="h-3.5 w-3.5" aria-hidden />
      ) : (
        <XCircle className="h-3.5 w-3.5" aria-hidden />
      )}
      {ok ? okLabel : errorLabel}
    </Badge>
  )
}
