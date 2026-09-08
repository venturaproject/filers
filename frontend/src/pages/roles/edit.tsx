import { AuthenticatedLayout } from "@/layouts"
import { ChevronLeft, Shield, Loader2 } from "lucide-react"
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
import { Checkbox } from "@/components/ui/checkbox"
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion"
import { Main } from "@/components/layout"
import { useI18n } from "@/i18n/context"
import { toast } from "sonner"
import { useState, useEffect } from "react"
import { useNavigate } from "react-router-dom"
import { pathFor } from "@/lib/app-routes"
import { useQueryClient } from "@tanstack/react-query"
import { PageProps, GroupedPermissions } from "@/types"
import { Badge } from "@/components/ui/badge"
import { rolesApi } from "@/services/roles-api"

interface Permission {
  id: number
  name: string
  guard_name: string
}

interface Role {
  id: number
  name: string
  guard_name: string
  permissions?: Permission[]
  permissions_count?: number
  users_count?: number
}

interface EditRolePageProps extends PageProps {
  role: Role
  permissions: Permission[]
  groupedPermissions: GroupedPermissions
  rolePermissions: number[]
}

export default function EditRole({
  role,
  groupedPermissions,
  rolePermissions,
}: EditRolePageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const fullAccessRoles = ['admin']
  const hasFullAccessRole = fullAccessRoles.includes(role.name)

  const [name, setName] = useState(role.name)
  const [selectedPermissions, setSelectedPermissions] = useState<number[]>(rolePermissions)
  const [isSubmitting, setIsSubmitting] = useState(false)

  useEffect(() => {
    setSelectedPermissions(rolePermissions)
  }, [rolePermissions])

  const handlePermissionChange = (permissionId: number, checked: boolean) => {
    if (hasFullAccessRole) return

    if (checked) {
      setSelectedPermissions([...selectedPermissions, permissionId])
    } else {
      setSelectedPermissions(selectedPermissions.filter(id => id !== permissionId))
    }
  }

  const handleSelectAllInGroup = (resource: string, checked: boolean) => {
    if (hasFullAccessRole) return

    const groupPerms = groupedPermissions[resource]?.permissions.map(p => p.id) || []

    if (checked) {
      const newPerms = [...new Set([...selectedPermissions, ...groupPerms])]
      setSelectedPermissions(newPerms)
    } else {
      setSelectedPermissions(selectedPermissions.filter(id => !groupPerms.includes(id)))
    }
  }

  const isGroupFullySelected = (resource: string) => {
    if (hasFullAccessRole) return true
    const groupPerms = groupedPermissions[resource]?.permissions.map(p => p.id) || []
    return groupPerms.every(id => selectedPermissions.includes(id))
  }

  const isGroupPartiallySelected = (resource: string) => {
    if (hasFullAccessRole) return false
    const groupPerms = groupedPermissions[resource]?.permissions.map(p => p.id) || []
    const selectedCount = groupPerms.filter(id => selectedPermissions.includes(id)).length
    return selectedCount > 0 && selectedCount < groupPerms.length
  }

  const onSubmit = async () => {
    if (!name.trim()) {
      toast.error(t('role_name_required') || 'Role name is required.')
      return
    }

    setIsSubmitting(true)
    try {
      await rolesApi.update(role.id, {
        name: name.trim(),
        permissions: selectedPermissions,
      })
      toast.success(t('role_updated') || 'Role updated successfully.')
      queryClient.invalidateQueries({ queryKey: ['roles'] })
      navigate(pathFor('admin.roles.index'))
    } catch (error: any) {
      if (error.response?.data) {
        toast.error(Object.values(error.response.data)[0] as string || t('error_updating_role') || 'Error updating role.')
      } else {
        toast.error(t('please_try_again') || 'Error. Please try again.')
      }
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <AuthenticatedLayout title={`${t('edit_role') || 'Edit Role'}: ${role.name}`}>
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
            <h1 className="text-xl font-semibold">{t('edit_role') || 'Edit Role'}</h1>
            {hasFullAccessRole && (
              <Badge variant="secondary">{t('system_role') || 'System Role'}</Badge>
            )}
          </div>

          <div className="grid gap-4 md:grid-cols-[1fr_2fr]">
            {/* Role Name Card */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Shield className="h-5 w-5" />
                  {t('role_details') || 'Role Details'}
                </CardTitle>
                <CardDescription>
                  {hasFullAccessRole
                    ? t('superadmin_all_permissions') || 'This role has full access to all permissions.'
                    : t('edit_role_name_description') || 'Update the role name and permissions.'}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="name">{t('role_name') || 'Role Name'}</Label>
                  <Input
                    id="name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder={t('enter_role_name_placeholder') || 'e.g., Content Manager'}
                    disabled={hasFullAccessRole}
                  />
                </div>

                <div className="pt-4">
                  <p className="text-sm text-muted-foreground">
                    {hasFullAccessRole
                      ? (t('all_permissions_label') || 'All permissions (via Gate::before bypass)')
                      : `${t('selected_permissions') || 'Selected permissions'}: ${selectedPermissions.length}`}
                  </p>
                </div>

                <Button
                  onClick={onSubmit}
                  disabled={isSubmitting || hasFullAccessRole}
                  className="w-full"
                >
                  {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {isSubmitting ? (t('saving') || 'Saving...') : (t('save_changes') || 'Save Changes')}
                </Button>
              </CardContent>
            </Card>

            {/* Permissions Card */}
            <Card>
              <CardHeader>
                <CardTitle>{t('permissions') || 'Permissions'}</CardTitle>
                <CardDescription>
                  {hasFullAccessRole
                    ? t('superadmin_all_permissions') || 'This role automatically has all permissions.'
                    : t('select_permissions_for_role') || 'Select the permissions for this role.'}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <Accordion type="multiple" className="w-full">
                  {Object.values(groupedPermissions).map((group) => (
                    <AccordionItem value={group.key} key={group.key}>
                      <AccordionTrigger className="hover:no-underline">
                        <div className="flex items-center gap-2">
                          <Checkbox
                            checked={isGroupFullySelected(group.key)}
                            ref={(el) => {
                              if (el) {
                                (el as HTMLButtonElement & { indeterminate: boolean }).indeterminate = isGroupPartiallySelected(group.key)
                              }
                            }}
                            onCheckedChange={(checked) => {
                              handleSelectAllInGroup(group.key, checked as boolean)
                            }}
                            onClick={(e) => e.stopPropagation()}
                            disabled={hasFullAccessRole}
                          />
                          <span className="font-medium">
                            {group.label}
                          </span>
                          <span className="text-muted-foreground text-sm">
                            ({group.permissions.length})
                          </span>
                        </div>
                      </AccordionTrigger>
                      <AccordionContent>
                        <div className="grid gap-2 pl-6 pt-2">
                          {group.permissions.map((permission) => (
                            <div key={permission.id} className="flex items-center space-x-2">
                              <Checkbox
                                id={String(permission.id)}
                                checked={hasFullAccessRole || selectedPermissions.includes(permission.id)}
                                onCheckedChange={(checked) => {
                                  handlePermissionChange(permission.id, checked as boolean)
                                }}
                                disabled={hasFullAccessRole}
                              />
                              <Label
                                htmlFor={String(permission.id)}
                                className="text-sm font-normal cursor-pointer"
                              >
                                {permission.actionLabel}
                              </Label>
                            </div>
                          ))}
                        </div>
                      </AccordionContent>
                    </AccordionItem>
                  ))}
                </Accordion>
              </CardContent>
            </Card>
          </div>
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
