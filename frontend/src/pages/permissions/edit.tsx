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

interface Permission {
  id: number
  name: string
  guard_name: string
}

interface EditPermissionPageProps extends PageProps {
  permission: Permission
}

export default function EditPermission({ permission }: EditPermissionPageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [name, setName] = useState(permission.name)
  const [isSubmitting, setIsSubmitting] = useState(false)

  const onSubmit = async () => {
    if (!name.trim()) {
      toast.error(t('permission_name_required') || 'Permission name is required.')
      return
    }

    if (!/^[a-z0-9.]+$/.test(name)) {
      toast.error(t('permission_name_format_hint') || 'Only lowercase letters, numbers, and dots allowed.')
      return
    }

    setIsSubmitting(true)
    try {
      await permissionsApi.update(permission.id, {
        name: name.trim(),
      })
      toast.success(t('permission_updated') || 'Permission updated successfully.')
      queryClient.invalidateQueries({ queryKey: ['permissions'] })
      navigate(pathFor('admin.permissions.index'))
    } catch (error: any) {
      if (error.response?.data) {
        toast.error(Object.values(error.response.data)[0] as string || t('error_updating_permission') || 'Error updating permission.')
      } else {
        toast.error(t('please_try_again') || 'Error. Please try again.')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  const group = name.split('.')[0]
  const action = name.split('.')[1] || ''

  return (
    <AuthenticatedLayout title={`${t('edit_permission') || 'Edit Permission'}: ${permission.name}`}>
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
            <h1 className="text-xl font-semibold">{t('edit_permission') || 'Edit Permission'}</h1>
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
                  {t('edit_permission_description') || 'Edit the permission name. Uses the format: group.action'}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="name">{t('current_permission') || 'Current Permission'}</Label>
                  <Input
                    id="name"
                    value={name}
                    onChange={(e) => setName(e.target.value.toLowerCase())}
                    placeholder={t('permission_format_hint') || 'e.g., users.view'}
                  />
                </div>

                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <p className="text-muted-foreground">{t('original_name') || 'Original name'}</p>
                    <p className="font-medium">{permission.name}</p>
                  </div>
                  <div>
                    <p className="text-muted-foreground">{t('guard') || 'Guard'}</p>
                    <p className="font-medium">{permission.guard_name}</p>
                  </div>
                </div>

                <div className="text-xs text-muted-foreground space-y-1 bg-yellow-50 p-3 rounded-lg border border-yellow-200">
                  <p className="font-semibold text-yellow-800">{t('warning') || 'Warning'}:</p>
                  <p>{t('permission_rename_warning') || 'Changing the permission name will affect all roles that use this permission.'}</p>
                </div>

                <Button onClick={onSubmit} disabled={isSubmitting || !name.trim()} className="w-full">
                  {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {isSubmitting ? (t('saving') || 'Saving...') : (t('save_changes') || 'Save Changes')}
                </Button>
              </CardContent>
            </Card>

            {/* Info Card */}
            <Card>
              <CardHeader>
                <CardTitle>{t('summary') || 'Summary'}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <p className="text-muted-foreground">{t('group') || 'Group'}</p>
                    <p className="font-medium">{group || '-'}</p>
                  </div>
                  <div>
                    <p className="text-muted-foreground">{t('action') || 'Action'}</p>
                    <p className="font-medium">{action || '-'}</p>
                  </div>
                </div>
                <div className="p-4 bg-muted rounded-lg">
                  <p className="text-sm text-muted-foreground">Permission</p>
                  <p className="font-mono text-xl font-semibold mt-1">{name}</p>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
