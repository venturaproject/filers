import { AuthenticatedLayout } from '@/layouts'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Main } from '@/components/layout/main'
import { MetricStatCard } from '@/components/metric-stat-card'
import NotificationList from '@/components/notification/notification-list'
import { Clock, FileSpreadsheet, Layers, Timer } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import type { JobSummary } from '@/services/files-api'
import {
  KIND_LABEL,
  ORIGIN_LABEL,
  STATUS_LABEL,
  STATUS_VARIANT,
  formatDuration,
} from '@/pages/process/jobs-columns'

// ── Types ───────────────────────────────────────────────────────────────────

interface Processings {
  total: number
  last_24h: number
  total_rows: number
  by_kind: Record<string, number>
  by_origin: Record<string, number>
  by_status: Record<string, number>
  avg_ms: number | null
  p95_ms: number | null
}

export interface DashboardData {
  processings: Processings
  in_flight: { pending: number; running: number; failed: number }
  counts: { users: number; roles: number; api_clients: number }
  recent: JobSummary[]
}

export const EMPTY_DASHBOARD: DashboardData = {
  processings: {
    total: 0,
    last_24h: 0,
    total_rows: 0,
    by_kind: {},
    by_origin: {},
    by_status: {},
    avg_ms: null,
    p95_ms: null,
  },
  in_flight: { pending: 0, running: 0, failed: 0 },
  counts: { users: 0, roles: 0, api_clients: 0 },
  recent: [],
}

// ── Breakdown bars ──────────────────────────────────────────────────────────

function Breakdown({
  map,
  label,
}: {
  map: Record<string, number>
  label: (k: string) => string
}) {
  const entries = Object.entries(map).sort((a, b) => b[1] - a[1])
  const max = Math.max(1, ...entries.map(([, v]) => v))

  if (entries.length === 0) {
    return <p className="text-sm text-muted-foreground">Sin datos todavía.</p>
  }

  return (
    <div className="space-y-3">
      {entries.map(([key, value]) => (
        <div key={key} className="space-y-1">
          <div className="flex justify-between text-sm">
            <span>{label(key)}</span>
            <span className="tabular-nums text-muted-foreground">
              {value.toLocaleString('es-ES')}
            </span>
          </div>
          <div className="h-2 rounded bg-muted">
            <div
              className="h-2 rounded bg-primary"
              style={{ width: `${(value / max) * 100}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  )
}

// ── Page ────────────────────────────────────────────────────────────────────

export default function Dashboard({ data }: { data: DashboardData }) {
  const navigate = useNavigate()
  const { processings: p, in_flight, counts, recent } = data

  return (
    <AuthenticatedLayout title="Dashboard">
      <Main>
        <div className="mb-2 flex items-center justify-between space-y-2">
          <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
        </div>

        <Tabs orientation="vertical" defaultValue="overview" className="space-y-4">
          <div className="w-full overflow-x-auto pb-2">
            <TabsList>
              <TabsTrigger value="overview">Resumen</TabsTrigger>
              <TabsTrigger value="notifications">Notificaciones</TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="overview" className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <MetricStatCard
                title="Procesamientos"
                value={p.total}
                subtitle={`${p.total_rows.toLocaleString('es-ES')} filas en total`}
                icon={FileSpreadsheet}
                sparklineColor="#6366f1"
              />
              <MetricStatCard
                title="Últimas 24 h"
                value={p.last_24h}
                subtitle="Archivos procesados"
                icon={Clock}
                sparklineColor="#10b981"
              />
              <MetricStatCard
                title="En cola"
                value={in_flight.pending + in_flight.running}
                subtitle={`${in_flight.running} procesando · ${in_flight.failed} con error`}
                icon={Layers}
                sparklineColor="#f59e0b"
              />
              <MetricStatCard
                title="Latencia media"
                value={p.avg_ms != null ? formatDuration(p.avg_ms) : '—'}
                subtitle={p.p95_ms != null ? `p95 ${formatDuration(p.p95_ms)}` : 'Sin datos'}
                icon={Timer}
                sparklineColor="#3b82f6"
              />
            </div>

            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <Card>
                <CardHeader>
                  <CardTitle>Por origen</CardTitle>
                  <CardDescription>Cómo se llamó al procesamiento</CardDescription>
                </CardHeader>
                <CardContent>
                  <Breakdown
                    map={p.by_origin}
                    label={(k) => (ORIGIN_LABEL as Record<string, string>)[k] ?? k}
                  />
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Por modo</CardTitle>
                  <CardDescription>Síncrono vs. batch</CardDescription>
                </CardHeader>
                <CardContent>
                  <Breakdown
                    map={p.by_kind}
                    label={(k) => (KIND_LABEL as Record<string, string>)[k] ?? k}
                  />
                </CardContent>
              </Card>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <MetricStatCard
                title="Usuarios"
                value={counts.users}
                subtitle="Con acceso al panel"
                icon={Layers}
                sparklineColor="#8b5cf6"
              />
              <MetricStatCard
                title="Roles"
                value={counts.roles}
                subtitle="Definidos"
                icon={Layers}
                sparklineColor="#ec4899"
              />
              <MetricStatCard
                title="Clientes API"
                value={counts.api_clients}
                subtitle="Activos"
                icon={Layers}
                sparklineColor="#14b8a6"
              />
            </div>

            <Card>
              <CardHeader>
                <CardTitle>Actividad reciente</CardTitle>
                <CardDescription>Últimos archivos procesados</CardDescription>
              </CardHeader>
              <CardContent className="p-0">
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Archivo</TableHead>
                        <TableHead>Modo</TableHead>
                        <TableHead>Origen</TableHead>
                        <TableHead>Estado</TableHead>
                        <TableHead className="text-right">Filas</TableHead>
                        <TableHead>Creado</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {recent.map((j) => (
                        <TableRow
                          key={j.id}
                          className="cursor-pointer"
                          onClick={() => navigate('/admin/jobs')}
                        >
                          <TableCell className="max-w-[220px] truncate font-medium">
                            {j.label ?? j.id.slice(0, 8)}
                          </TableCell>
                          <TableCell>
                            <Badge variant="secondary" className="font-normal">
                              {KIND_LABEL[j.kind]}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className="font-normal">
                              {ORIGIN_LABEL[j.origin]}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <Badge variant={STATUS_VARIANT[j.status]}>
                              {STATUS_LABEL[j.status]}
                            </Badge>
                          </TableCell>
                          <TableCell className="text-right tabular-nums">
                            {j.total_rows.toLocaleString('es-ES')}
                          </TableCell>
                          <TableCell className="text-xs text-muted-foreground">
                            {new Date(j.created_at).toLocaleString('es-ES')}
                          </TableCell>
                        </TableRow>
                      ))}
                      {recent.length === 0 && (
                        <TableRow>
                          <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                            Sin procesamientos todavía
                          </TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="notifications" className="space-y-4">
            <div className="max-w-2xl">
              <NotificationList />
            </div>
          </TabsContent>
        </Tabs>
      </Main>
    </AuthenticatedLayout>
  )
}
