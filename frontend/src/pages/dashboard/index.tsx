import { AuthenticatedLayout } from '@/layouts'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Main } from '@/components/layout/main'
import { MetricStatCard } from '@/components/metric-stat-card'
import NotificationList from '@/components/notification/notification-list'
import { FileSpreadsheet, Upload, CheckCircle2, Timer } from 'lucide-react'

export default function Dashboard() {
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
                title="Archivos procesados"
                value="—"
                subtitle="Total histórico"
                icon={FileSpreadsheet}
                sparklineColor="#6366f1"
              />
              <MetricStatCard
                title="Subidos hoy"
                value="—"
                subtitle="Últimas 24 h"
                icon={Upload}
                sparklineColor="#10b981"
              />
              <MetricStatCard
                title="Trabajos completados"
                value="—"
                subtitle="Procesamiento batch"
                icon={CheckCircle2}
                sparklineColor="#f59e0b"
              />
              <MetricStatCard
                title="Latencia media"
                value="—"
                subtitle="Tiempo de parsing"
                icon={Timer}
                sparklineColor="#3b82f4"
              />
            </div>

            <Card>
              <CardHeader>
                <CardTitle>Bienvenido a Filers</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-muted-foreground text-sm">
                  Procesa archivos Excel y CSV de manera individual o masiva.{' '}
                  <a href="/admin/process" className="underline underline-offset-2">Procesar un archivo</a>
                  {' '}o consulta el estado de un{' '}
                  <a href="/admin/jobs" className="underline underline-offset-2">trabajo batch</a>.
                </p>
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
