import { BadgeCheck, Ban, CircleMinus, Mail, type LucideIcon } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { UserStatus } from './data/schema'

type StatusMeta = { label: string; icon: LucideIcon; className: string }

export const USER_STATUS_META: Record<UserStatus, StatusMeta> = {
  active: {
    label: 'Activo',
    icon: BadgeCheck,
    className: 'border-emerald-500/25 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  },
  inactive: {
    label: 'Inactivo',
    icon: CircleMinus,
    className: 'border-border bg-muted text-muted-foreground',
  },
  invited: {
    label: 'Invitado',
    icon: Mail,
    className: 'border-sky-500/25 bg-sky-500/10 text-sky-700 dark:text-sky-300',
  },
  suspended: {
    label: 'Suspendido',
    icon: Ban,
    className: 'border-destructive/25 bg-destructive/10 text-destructive',
  },
}

export function UserStatusBadge({
  status,
  label,
  className,
}: {
  status: UserStatus
  /** i18n label from the caller; falls back to the Spanish default */
  label?: string
  className?: string
}) {
  const meta = USER_STATUS_META[status] ?? USER_STATUS_META.inactive
  const Icon = meta.icon
  return (
    <Badge variant="outline" className={cn('gap-1.5 font-medium', meta.className, className)}>
      <Icon className="h-3.5 w-3.5" aria-hidden />
      {label ?? meta.label}
    </Badge>
  )
}
