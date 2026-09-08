import { useEffect, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Copy, Trash2, Plus, ShieldOff, Check, Eye, EyeOff, KeyRound, SlidersHorizontal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/checkbox'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { apiClientsApi, ApiClientRecord, AVAILABLE_SCOPES } from '@/services/api-clients-api'

const DEFAULT_RATE_LIMIT_HINT = 'p. ej. 30/60 (30 peticiones cada 60 s)'

// ── Copy button ───────────────────────────────────────────────────────────────

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)
  const handleCopy = () => {
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  return (
    <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={handleCopy} title="Copiar">
      {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
    </Button>
  )
}

// ── Secret reveal modal ────────────────────────────────────────────────────────

interface SecretModalProps {
  open: boolean
  onClose: () => void
  clientId: string
  secret: string
  title?: string
}

function SecretModal({ open, onClose, clientId, secret, title = 'Cliente API creado' }: SecretModalProps) {
  const [revealed, setRevealed] = useState(false)
  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            Guarda el secreto ahora — no podrás verlo de nuevo.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-1">
            <Label className="text-xs text-muted-foreground">Client ID</Label>
            <div className="flex items-center gap-2 rounded-md border bg-muted/40 px-3 py-2 font-mono text-sm">
              <span className="flex-1">{clientId}</span>
              <CopyButton text={clientId} />
            </div>
          </div>

          <div className="space-y-1">
            <Label className="text-xs text-muted-foreground">Client Secret</Label>
            <div className="flex items-center gap-2 rounded-md border border-yellow-400 bg-yellow-50 px-3 py-2 font-mono text-sm dark:bg-yellow-950/30">
              <span className="flex-1 break-all">
                {revealed ? secret : '•'.repeat(Math.min(secret.length, 40))}
              </span>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 shrink-0"
                onClick={() => setRevealed((v) => !v)}
              >
                {revealed ? <EyeOff className="h-3 w-3" /> : <Eye className="h-3 w-3" />}
              </Button>
              <CopyButton text={secret} />
            </div>
            <p className="text-xs text-yellow-600 dark:text-yellow-400">
              Este secreto no volverá a mostrarse. Cópialo ahora.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button onClick={onClose}>Entendido</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ── Create dialog ─────────────────────────────────────────────────────────────

interface CreateDialogProps {
  open: boolean
  onClose: () => void
  onCreate: (name: string, scopes: string[]) => Promise<void>
  loading: boolean
}

function CreateDialog({ open, onClose, onCreate, loading }: CreateDialogProps) {
  const [name, setName] = useState('')
  const [scopes, setScopes] = useState<string[]>([])

  const toggleScope = (scope: string) => {
    if (scope === '*') {
      setScopes((prev) => (prev.includes('*') ? [] : ['*']))
      return
    }
    setScopes((prev) =>
      prev.includes(scope)
        ? prev.filter((s) => s !== scope)
        : [...prev.filter((s) => s !== '*'), scope]
    )
  }

  const handleClose = () => {
    setName('')
    setScopes([])
    onClose()
  }

  const handleSubmit = async () => {
    if (!name.trim()) { toast.error('El nombre es obligatorio'); return }
    if (scopes.length === 0) { toast.error('Selecciona al menos un scope'); return }
    await onCreate(name.trim(), scopes)
    setName('')
    setScopes([])
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && handleClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Nuevo cliente API</DialogTitle>
          <DialogDescription>
            Crea un cliente para acceder a la API externa con credenciales propias.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-1">
            <Label htmlFor="client-name">Nombre del sistema</Label>
            <Input
              id="client-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="ERP, Sistema externo…"
              onKeyDown={(e) => e.key === 'Enter' && !loading && handleSubmit()}
            />
          </div>

          <div className="space-y-2">
            <Label>Scopes (permisos)</Label>
            <div className="space-y-2 rounded-md border p-3">
              {AVAILABLE_SCOPES.map((s) => (
                <div key={s.value} className="flex items-center gap-2">
                  <Checkbox
                    id={`scope-${s.value}`}
                    checked={scopes.includes(s.value) || (s.value !== '*' && scopes.includes('*'))}
                    onCheckedChange={() => toggleScope(s.value)}
                    disabled={s.value !== '*' && scopes.includes('*')}
                  />
                  <Label htmlFor={`scope-${s.value}`} className="cursor-pointer text-sm font-normal">
                    {s.label}
                  </Label>
                </div>
              ))}
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose}>Cancelar</Button>
          <Button onClick={handleSubmit} disabled={loading}>
            {loading ? 'Creando…' : 'Crear cliente'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ── Edit limits dialog ────────────────────────────────────────────────────────

interface EditLimitsDialogProps {
  client: ApiClientRecord | null
  onClose: () => void
  onSave: (payload: { rate_limit: string | null; monthly_page_quota: number | null }) => Promise<void>
  loading: boolean
}

function EditLimitsDialog({ client, onClose, onSave, loading }: EditLimitsDialogProps) {
  const [rateLimit, setRateLimit] = useState('')
  const [quota, setQuota] = useState('')

  useEffect(() => {
    if (client) {
      setRateLimit(client.rate_limit ?? '')
      setQuota(client.monthly_page_quota != null ? String(client.monthly_page_quota) : '')
    }
  }, [client])

  const handleSubmit = async () => {
    const rl = rateLimit.trim()
    if (rl && !/^\d+\/\d+$/.test(rl)) {
      toast.error('El límite debe tener el formato «n/segundos», p. ej. 30/60')
      return
    }
    const q = quota.trim()
    if (q && !/^\d+$/.test(q)) {
      toast.error('La cuota mensual debe ser un número de páginas')
      return
    }
    await onSave({
      rate_limit: rl || null,
      monthly_page_quota: q ? Number(q) : null,
    })
  }

  return (
    <Dialog open={!!client} onOpenChange={(v) => !v && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Límites de «{client?.name}»</DialogTitle>
          <DialogDescription>
            Deja un campo vacío para usar el valor por defecto del servidor.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="space-y-1">
            <Label htmlFor="client-rate-limit">Límite de peticiones</Label>
            <Input
              id="client-rate-limit"
              value={rateLimit}
              onChange={(e) => setRateLimit(e.target.value)}
              placeholder={DEFAULT_RATE_LIMIT_HINT}
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="client-quota">Cuota mensual de páginas</Label>
            <Input
              id="client-quota"
              type="number"
              min={0}
              value={quota}
              onChange={(e) => setQuota(e.target.value)}
              placeholder="Sin límite"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancelar</Button>
          <Button onClick={handleSubmit} disabled={loading}>
            {loading ? 'Guardando…' : 'Guardar'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ── Per-client usage cell ─────────────────────────────────────────────────────

function UsageCell({ clientId }: { clientId: string }) {
  const { data, isLoading } = useQuery({
    queryKey: ['api-client-usage', clientId],
    queryFn: () => apiClientsApi.usage(clientId),
  })

  if (isLoading || !data) {
    return <span className="text-xs text-muted-foreground">—</span>
  }

  return (
    <div className="space-y-0.5 text-xs text-muted-foreground">
      <div>
        <span className="font-medium text-foreground">{data.pages.toLocaleString()}</span>
        {data.monthly_page_quota
          ? ` / ${data.monthly_page_quota.toLocaleString()} págs.`
          : ' págs. (sin cuota)'}
      </div>
      <div>{data.rate_limit ?? '—'} · {data.period}</div>
    </div>
  )
}

// ── Main tab ──────────────────────────────────────────────────────────────────

export function ApiClientsTab() {
  const queryClient = useQueryClient()
  const [createOpen, setCreateOpen] = useState(false)
  const [secretModal, setSecretModal] = useState<{ clientId: string; secret: string; title: string } | null>(null)
  const [confirmRevoke, setConfirmRevoke] = useState<ApiClientRecord | null>(null)
  const [editLimits, setEditLimits] = useState<ApiClientRecord | null>(null)
  const [confirmRotate, setConfirmRotate] = useState<ApiClientRecord | null>(null)

  const { data: clients = [], isLoading } = useQuery({
    queryKey: ['api-clients'],
    queryFn: () => apiClientsApi.list(),
  })

  const createMutation = useMutation({
    mutationFn: ({ name, scopes }: { name: string; scopes: string[] }) =>
      apiClientsApi.create({ name, scopes }),
    onSuccess: (res) => {
      queryClient.invalidateQueries({ queryKey: ['api-clients'] })
      setCreateOpen(false)
      setSecretModal({ clientId: res.client.client_id, secret: res.secret, title: 'Cliente API creado' })
    },
    onError: () => toast.error('Error al crear el cliente API'),
  })

  const updateMutation = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: { rate_limit: string | null; monthly_page_quota: number | null } }) =>
      apiClientsApi.update(id, payload),
    onSuccess: (_res, { id }) => {
      queryClient.invalidateQueries({ queryKey: ['api-clients'] })
      queryClient.invalidateQueries({ queryKey: ['api-client-usage', id] })
      toast.success('Límites actualizados')
      setEditLimits(null)
    },
    onError: () => toast.error('No se pudieron actualizar los límites'),
  })

  const rotateMutation = useMutation({
    mutationFn: (id: string) => apiClientsApi.rotate(id),
    onSuccess: (res) => {
      queryClient.invalidateQueries({ queryKey: ['api-clients'] })
      setConfirmRotate(null)
      setSecretModal({ clientId: res.client.client_id, secret: res.secret, title: 'Secreto rotado' })
    },
    onError: () => toast.error('No se pudo rotar el secreto'),
  })

  const revokeMutation = useMutation({
    mutationFn: (id: string) => apiClientsApi.revoke(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-clients'] })
      toast.success('Cliente revocado')
      setConfirmRevoke(null)
    },
    onError: () => toast.error('Error al revocar el cliente'),
  })

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-start justify-between space-y-0">
          <div>
            <CardTitle>Clientes de API externa</CardTitle>
            <CardDescription className="mt-1">
              Gestiona los clientes que acceden a la API externa mediante Bearer token
            </CardDescription>
          </div>
          <Button size="sm" onClick={() => setCreateOpen(true)}>
            <Plus className="mr-1 h-4 w-4" />
            Nuevo cliente
          </Button>
        </CardHeader>

        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Nombre</TableHead>
                <TableHead>Client ID</TableHead>
                <TableHead>Scopes</TableHead>
                <TableHead>Límites y uso</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead>Último uso</TableHead>
                <TableHead className="text-right">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={7} className="py-10 text-center text-sm text-muted-foreground">
                    Cargando…
                  </TableCell>
                </TableRow>
              ) : clients.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="py-10 text-center text-sm text-muted-foreground">
                    No hay clientes API. Crea el primero.
                  </TableCell>
                </TableRow>
              ) : (
                clients.map((client) => (
                  <TableRow key={client.id}>
                    <TableCell className="font-medium">{client.name}</TableCell>

                    <TableCell>
                      <div className="flex items-center gap-1 font-mono text-xs text-muted-foreground">
                        <span>{client.client_id}</span>
                        <CopyButton text={client.client_id} />
                      </div>
                    </TableCell>

                    <TableCell>
                      <div className="flex flex-wrap gap-1">
                        {client.scopes.map((s) => (
                          <Badge key={s} variant="secondary" className="text-xs">
                            {s}
                          </Badge>
                        ))}
                      </div>
                    </TableCell>

                    <TableCell>
                      <UsageCell clientId={client.id} />
                    </TableCell>

                    <TableCell>
                      {client.active ? (
                        <Badge variant="outline" className="border-green-500 text-green-600">
                          Activo
                        </Badge>
                      ) : (
                        <Badge variant="outline" className="border-red-400 text-red-500">
                          Revocado
                        </Badge>
                      )}
                    </TableCell>

                    <TableCell className="text-sm text-muted-foreground">
                      {client.last_used_at
                        ? new Date(client.last_used_at).toLocaleString()
                        : 'Nunca'}
                    </TableCell>

                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="text-muted-foreground"
                          disabled={!client.active}
                          onClick={() => setEditLimits(client)}
                          title="Editar límites"
                        >
                          <SlidersHorizontal className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="text-muted-foreground"
                          disabled={!client.active}
                          onClick={() => setConfirmRotate(client)}
                          title="Rotar secreto"
                        >
                          <KeyRound className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="text-muted-foreground hover:text-destructive"
                          disabled={!client.active}
                          onClick={() => setConfirmRevoke(client)}
                          title={client.active ? 'Revocar cliente' : 'Ya revocado'}
                        >
                          {client.active ? (
                            <Trash2 className="h-4 w-4" />
                          ) : (
                            <ShieldOff className="h-4 w-4" />
                          )}
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <CreateDialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        loading={createMutation.isPending}
        onCreate={async (name, scopes) => { await createMutation.mutateAsync({ name, scopes }) }}
      />

      <EditLimitsDialog
        client={editLimits}
        onClose={() => setEditLimits(null)}
        loading={updateMutation.isPending}
        onSave={async (payload) => {
          if (editLimits) await updateMutation.mutateAsync({ id: editLimits.id, payload })
        }}
      />

      {secretModal && (
        <SecretModal
          open
          onClose={() => setSecretModal(null)}
          clientId={secretModal.clientId}
          secret={secretModal.secret}
          title={secretModal.title}
        />
      )}

      <ConfirmDialog
        open={!!confirmRevoke}
        onOpenChange={(v) => !v && setConfirmRevoke(null)}
        title="Revocar cliente API"
        desc={`¿Revocar "${confirmRevoke?.name}"? Sus tokens dejarán de funcionar de inmediato.`}
        confirmText="Revocar"
        destructive
        handleConfirm={() => confirmRevoke && revokeMutation.mutate(confirmRevoke.id)}
      />

      <ConfirmDialog
        open={!!confirmRotate}
        onOpenChange={(v) => !v && setConfirmRotate(null)}
        title="Rotar secreto"
        desc={`¿Rotar el secreto de "${confirmRotate?.name}"? El secreto actual y todos sus tokens dejarán de funcionar de inmediato.`}
        confirmText="Rotar secreto"
        destructive
        handleConfirm={() => confirmRotate && rotateMutation.mutate(confirmRotate.id)}
      />
    </>
  )
}
