import { useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { auditApi } from '@/services/audit-api'

const PER_PAGE = 50

function actionTone(action: string): string {
  if (action.endsWith('.failed') || action.endsWith('.locked') || action.endsWith('.blocked'))
    return 'border-red-400 text-red-500'
  if (action.endsWith('.revoked') || action.endsWith('.deleted')) return 'border-amber-400 text-amber-600'
  if (action.endsWith('.ok') || action.endsWith('.created')) return 'border-green-500 text-green-600'
  return 'border-muted-foreground/40 text-muted-foreground'
}

export default function AuditPage() {
  const [page, setPage] = useState(1)
  const [action, setAction] = useState('')
  const [actor, setActor] = useState('')

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ['audit', page, action, actor],
    queryFn: () =>
      auditApi.list({
        page,
        per_page: PER_PAGE,
        action: action.trim() || undefined,
        actor: actor.trim() || undefined,
      }),
    placeholderData: keepPreviousData,
  })

  const rows = data?.data ?? []
  const total = data?.total ?? 0
  const lastPage = Math.max(1, Math.ceil(total / PER_PAGE))

  return (
    <AuthenticatedLayout title="Auditoría">
      <Main>
        <div className="mb-4">
          <h1 className="text-2xl font-bold tracking-tight">Log de auditoría</h1>
          <p className="text-sm text-muted-foreground">
            Eventos de seguridad: inicios de sesión, cambios de usuarios y roles, y ciclo de vida de los clientes API.
          </p>
        </div>

        <Card>
          <CardHeader className="gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <CardTitle>Eventos</CardTitle>
                <CardDescription className="mt-1">
                  {total.toLocaleString()} evento{total === 1 ? '' : 's'}
                </CardDescription>
              </div>
              <div className="flex gap-2">
                <Input
                  className="h-9 w-44"
                  placeholder="acción (p. ej. auth.login)"
                  value={action}
                  onChange={(e) => {
                    setAction(e.target.value)
                    setPage(1)
                  }}
                />
                <Input
                  className="h-9 w-44"
                  placeholder="actor (email / cliente)"
                  value={actor}
                  onChange={(e) => {
                    setActor(e.target.value)
                    setPage(1)
                  }}
                />
              </div>
            </div>
          </CardHeader>

          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-44">Fecha</TableHead>
                    <TableHead>Acción</TableHead>
                    <TableHead>Actor</TableHead>
                    <TableHead>Objetivo</TableHead>
                    <TableHead>IP</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {isLoading ? (
                    <TableRow>
                      <TableCell colSpan={5} className="py-10 text-center text-sm text-muted-foreground">
                        Cargando…
                      </TableCell>
                    </TableRow>
                  ) : rows.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5} className="py-10 text-center text-sm text-muted-foreground">
                        Sin eventos.
                      </TableCell>
                    </TableRow>
                  ) : (
                    rows.map((e) => (
                      <TableRow key={e.id}>
                        <TableCell className="text-xs text-muted-foreground">
                          {new Date(e.created_at).toLocaleString()}
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className={`font-mono text-xs ${actionTone(e.action)}`}>
                            {e.action}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-sm">
                          {e.actor_label ?? '—'}
                          <span className="ml-1 text-xs text-muted-foreground">({e.actor_type})</span>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">{e.target ?? '—'}</TableCell>
                        <TableCell className="font-mono text-xs text-muted-foreground">{e.ip ?? '—'}</TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>

            <div className="flex items-center justify-between border-t px-4 py-3 text-sm text-muted-foreground">
              <span>
                Página {page} de {lastPage}
                {isFetching ? ' · actualizando…' : ''}
              </span>
              <div className="flex gap-1">
                <Button
                  variant="outline"
                  size="icon"
                  className="h-8 w-8"
                  disabled={page <= 1}
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  className="h-8 w-8"
                  disabled={page >= lastPage}
                  onClick={() => setPage((p) => p + 1)}
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </Main>
    </AuthenticatedLayout>
  )
}
