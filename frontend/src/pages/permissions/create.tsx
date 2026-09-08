import { AuthenticatedLayout } from "@/layouts"
import { ChevronLeft, Lock, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Main } from "@/components/layout"
import { useI18n } from "@/i18n/context"
import { toast } from "sonner"
import { useState } from "react"
import { useNavigate } from "react-router-dom"
import { pathFor } from "@/lib/app-routes"
import { useQueryClient } from "@tanstack/react-query"
import { PageProps } from "@/types"
import { permissionsApi } from "@/services/permissions-api"

const COMMON_GROUPS = [
  'users', 'roles', 'permissions', 'ocr', 'documents', 'api_clients', 'settings', 'dashboard'
]

const COMMON_ACTIONS = [
  'view', 'create', 'edit', 'delete', 'export', 'import'
]

type CreatePermissionPageProps = PageProps

export default function CreatePermission(_props: CreatePermissionPageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [group, setGroup] = useState('')
  const [action, setAction] = useState('')
  const [customGroup, setCustomGroup] = useState('')
  const [customAction, setCustomAction] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)

  const permissionName = [group || customGroup, action || customAction].filter(Boolean).join('.')

  const handleGroupChange = (value: string) => {
    if (value === 'custom') {
      setGroup('')
    } else {
      setGroup(value)
      setCustomGroup('')
    }
  }

  const handleActionChange = (value: string) => {
    if (value === 'custom') {
      setAction('')
    } else {
      setAction(value)
      setCustomAction('')
    }
  }

  const onSubmit = async () => {
    const name = [group || customGroup, action || customAction].filter(Boolean).join('.')
    
    if (!name || name === '.') {
      t('permission_name_required')
      return
    }

    if (!/^[a-z0-9.]+$/.test(name)) {
      toast.error(t('permission_name_format_hint') || 'Only lowercase letters, numbers, and dots allowed.')
      return
    }

    setIsSubmitting(true)
    try {
      await permissionsApi.create({
        name,
        guard_name: 'api',
      })
      toast.success(t('permission_created') || 'Permission created successfully.')
      queryClient.invalidateQueries({ queryKey: ['permissions'] })
      navigate(pathFor('admin.permissions.index'))
    } catch (error: any) {
      if (error.response?.data) {
        toast.error(Object.values(error.response.data)[0] as string || t('error_creating_permission') || 'Error creating permission.')
      } else {
        toast.error(t('please_try_again') || 'Error. Please try again.')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <AuthenticatedLayout title={t('create_permission') || 'Create Permission'}>
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <div className="flex items-center gap-4">
            <Button
              variant="outline"
              size="icon"
              onClick={() => navigate(-1)}
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-xl font-semibold">{t('create_permission') || 'Create New Permission'}</h1>
          </div>

          <div className="grid gap-4 md:grid-cols-[1fr_2fr]">
            {/* Permission Details Card */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Lock className="h-5 w-5" />
                  {t('permission_details') || 'Permission Details'}
                </CardTitle>
                <CardDescription>
                  {t('create_permission_description') || 'Create a new permission using the format: group.action (e.g., users.view, posts.create)'}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="group">{t('group') || 'Group'}</Label>
                  <Input
                    id="group"
                    list="groups"
                    value={group || customGroup}
                    onChange={(e) => {
                      const val = e.target.value.toLowerCase()
                      if (COMMON_GROUPS.includes(val)) {
                        setGroup(val)
                        setCustomGroup('')
                      } else {
                        setCustomGroup(val)
                      }
                    }}
                    placeholder={t('select_a_group') || 'e.g., users'}
                  />
                  <datalist id="groups">
                    {COMMON_GROUPS.map(g => (
                      <option key={g} value={g} />
                    ))}
                  </datalist>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="action">{t('action') || 'Action'}</Label>
                  <Input
                    id="action"
                    list="actions"
                    value={action || customAction}
                    onChange={(e) => {
                      const val = e.target.value.toLowerCase()
                      if (COMMON_ACTIONS.includes(val)) {
                        setAction(val)
                        setCustomAction('')
                      } else {
                        setCustomAction(val)
                      }
                    }}
                    placeholder={t('select_an_action') || 'e.g., view'}
                  />
                  <datalist id="actions">
                    {COMMON_ACTIONS.map(a => (
                      <option key={a} value={a} />
                    ))}
                  </datalist>
                </div>

                <div className="pt-4">
                  <p className="text-sm text-muted-foreground">
                    {t('permission_preview_description') || 'Permission name'}:
                  </p>
                  <p className="font-mono text-lg font-semibold">{permissionName || '-'}</p>
                </div>

                <div className="text-xs text-muted-foreground space-y-1">
                  <p>{t('tips') || 'Tips'}:</p>
                  <p>{t('permission_format_hint') || 'Permissions follow the format: group.action'}</p>
                  <p>{t('permission_common_actions_hint') || 'Common actions include: view, create, edit, delete'}</p>
                </div>

                <Button onClick={onSubmit} disabled={isSubmitting || !permissionName} className="w-full">
                  {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {isSubmitting ? (t('creating') || 'Creating...') : (t('create_permission') || 'Create Permission')}
                </Button>
              </CardContent>
            </Card>

            {/* Preview Card */}
            <Card>
              <CardHeader>
                <CardTitle>{t('preview') || 'Preview'}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="p-4 bg-muted rounded-lg">
                  <p className="text-sm text-muted-foreground">{t('permission_name') || 'Permission name'}</p>
                  <p className="font-mono text-xl font-semibold mt-1">{permissionName || '-'}</p>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
