import { AuthenticatedLayout } from "@/layouts"
import { Main } from "@/components/layout"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { ChevronLeft, Pencil, Clock, Shield, KeyRound } from "lucide-react"
import { useNavigate } from "react-router-dom"
import { useI18n } from "@/i18n/context"
import { cn } from "@/lib/utils"
import { formatDistanceToNow, format } from "date-fns"
import { es } from "date-fns/locale"
import { PageProps } from "@/types"
import { callTypes } from "./data/data"
import { UserStatus } from "./data/schema"
import { pathFor } from "@/lib/app-routes"

interface ActivityLog {
  id: number
  accion: string
  descripcion: string | null
  ip: string | null
  ruta: string | null
  created_at: string
}

interface UserData {
  id: string
  name: string
  username: string | null
  email: string
  status: UserStatus
  role: string
  roles: string[]
  avatar: string | null
  lastActivity: string | null
  created_at?: string | null
}

interface ShowUserPageProps extends PageProps {
  user: UserData & { activityLogs?: ActivityLog[] }
}

function getInitials(name?: string | null): string {
  if (!name?.trim()) return 'U'

  return name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0] ?? '')
    .join('')
    .toUpperCase()
}

export default function ShowUser({
  user,
}: ShowUserPageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  
  const displayName = user.name || user.email
  const createdAt = user.created_at
  const statusCls = callTypes.get(user.status) ?? ''

  const statusLabels: Record<string, string> = {
    active: t('status_active') || 'Activo',
    inactive: t('status_inactive') || 'Inactivo',
    suspended: t('status_suspended') || 'Suspendido',
  }

  return (
    <AuthenticatedLayout title={displayName}>
      <Main>
        <div className="grid flex-1 items-start gap-6 md:gap-8 max-w-4xl mx-auto">
          <div className="flex items-center gap-4">
            <Button variant="outline" size="icon" onClick={() => navigate(pathFor('admin.users.index'))}>
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <div className="flex-1">
              <h2 className="text-2xl font-bold tracking-tight">{displayName || t('user') || 'Usuario'}</h2>
              <p className="text-muted-foreground text-sm">{user.email}</p>
            </div>
            <Button onClick={() => navigate(pathFor('admin.users.edit', user.id))}>
              <Pencil className="mr-2 h-4 w-4" />
              {t('edit') || 'Editar'}
            </Button>
          </div>

          {/* Profile card */}
          <Card>
            <CardHeader>
              <CardTitle>{t('user_profile') || 'Perfil de Usuario'}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex items-start gap-6">
                <Avatar className="h-20 w-20 shrink-0">
                  {user.avatar && <AvatarImage src={user.avatar} alt={displayName} />}
                  <AvatarFallback className="bg-primary/10 text-primary text-2xl font-bold">
                    {getInitials(displayName)}
                  </AvatarFallback>
                </Avatar>
                <div className="grid gap-3 flex-1">
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <p className="text-muted-foreground text-xs uppercase tracking-wide mb-1">{t('col_nombre') || 'Nombre'}</p>
                      <p className="font-medium">{displayName || '—'}</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground text-xs uppercase tracking-wide mb-1">{t('username') || 'Usuario'}</p>
                      <p className="font-mono">{user.username ?? '—'}</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground text-xs uppercase tracking-wide mb-1">{t('email') || 'Email'}</p>
                      <p>{user.email}</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground text-xs uppercase tracking-wide mb-1">{t('col_estado') || 'Estado'}</p>
                      <Badge variant="outline" className={cn('rounded-full capitalize', statusCls)}>
                        {statusLabels[user.status] ?? user.status}
                      </Badge>
                    </div>
                    <div>
                      <p className="text-muted-foreground text-xs uppercase tracking-wide mb-1">{t('col_fecha_alta') || 'Fecha Alta'}</p>
                      <p className="text-sm">{createdAt ? format(new Date(createdAt), 'dd/MM/yyyy', { locale: es }) : '—'}</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground text-xs uppercase tracking-wide mb-1">{t('col_last_activity') || 'Última Actividad'}</p>
                      <p className="text-sm">
                        {user.lastActivity
                          ? formatDistanceToNow(new Date(user.lastActivity), { addSuffix: true, locale: es })
                          : '—'}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Roles */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shield className="h-4 w-4" />
                {t('roles') || 'Roles'}
              </CardTitle>
            </CardHeader>
            <CardContent>
              {user.roles && user.roles.length > 0 ? (
                <div className="flex flex-wrap gap-2">
                  {user.roles.map((role) => (
                    <Badge key={role} variant="secondary">{role}</Badge>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">{t('no_roles_assigned') || 'Sin roles asignados'}</p>
              )}
            </CardContent>
          </Card>

          {/* Activity log */}
          {user.activityLogs && user.activityLogs.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Clock className="h-4 w-4" />
                  {t('user_activity_log') || 'Registro de actividad'}
                </CardTitle>
                <CardDescription>{t('user_activity_log_description') || 'Últimas acciones realizadas por el usuario'}</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-3">
                  {user.activityLogs.map((log) => (
                    <div key={log.id} className="flex items-start justify-between gap-3 py-2 border-b last:border-0">
                      <div className="min-w-0">
                        <p className="text-sm font-medium">{log.accion}</p>
                        {log.descripcion && (
                          <p className="text-xs text-muted-foreground">{log.descripcion}</p>
                        )}
                        {log.ip && (
                          <p className="text-xs text-muted-foreground font-mono">{log.ip} · {log.ruta}</p>
                        )}
                      </div>
                      <span className="text-xs text-muted-foreground whitespace-nowrap shrink-0">
                        {formatDistanceToNow(new Date(log.created_at), { addSuffix: true, locale: es })}
                      </span>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
