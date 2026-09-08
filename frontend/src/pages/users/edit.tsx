import { AuthenticatedLayout } from "@/layouts"
import { ChevronLeft, KeyRound, Loader2 } from "lucide-react"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Main } from "@/components/layout"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { editUserSchema, EditUserFormValues } from "@/schemas/user.schema"
import { useNavigate } from "react-router-dom"
import { Badge } from "@/components/ui/badge"
import { useI18n } from "@/i18n/context"
import { ScrollArea } from "@/components/ui/scroll-area"
import { generatePassword } from "@/lib/generate-password"
import { cn } from "@/lib/utils"
import { toast } from "sonner"
import { pathFor } from "@/lib/app-routes"
import { PageProps } from "@/types"
import { usersApi } from "@/services/users-api"

interface Role {
  id: number
  name: string
}

interface UserRole {
  id: number
  name: string
}

interface UserData {
  id: number | string
  name: string
  username: string | null
  email: string
  status: string
  created_at?: string | null
  roles: UserRole[]
}

interface EditUserPageProps extends PageProps {
  user: UserData
  roles: Role[]
}

export default function EditUser({
  user,
  roles = [],
}: EditUserPageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  
  const availableRoles = Array.isArray(roles) ? roles : []
  const displayName = user.name || user.email

  const { register, handleSubmit, watch, setValue, setError, formState: { errors, isSubmitting } } = useForm<EditUserFormValues>({
    resolver: zodResolver(editUserSchema),
    defaultValues: {
      name: user.name || '',
      username: user.username ?? '',
      email: user.email,
      password: '',
      roles: user.roles?.map(r => r.name) || [],
      role_ids: user.roles?.map(r => r.id) || [],
    },
  })

  const role_ids = watch('role_ids') || []
  const selectedRoleId = role_ids[0]?.toString() || ""

  const handleGeneratePassword = () => {
    setValue('password', generatePassword())
  }

  const handleRoleChange = (roleIdStr: string) => {
    const roleId = parseInt(roleIdStr)
    const role = availableRoles.find(r => r.id === roleId)
    if (role) {
      setValue('role_ids', [role.id])
      setValue('roles', [role.name])
    } else {
      setValue('role_ids', [])
      setValue('roles', [])
    }
  }

  const onSubmit = async (values: EditUserFormValues) => {
    try {
      const data: any = {
        name: values.name,
        username: values.username,
        email: values.email,
        role_ids: values.role_ids,
        status: user.status
      }
      
      if (values.password) {
        data.password = values.password
      }

      await usersApi.update(user.id, data)
      toast.success(t('user_updated') || 'Usuario actualizado correctamente')
      navigate(pathFor('admin.users.index'))
    } catch (error: any) {
      if (error.response?.data) {
        const serverErrors = error.response.data
        Object.entries(serverErrors).forEach(([key, message]) => {
          setError(key as any, { message: Array.isArray(message) ? message[0] : message as string })
        })
      }
      toast.error(t('please_try_again') || 'Error al actualizar el usuario')
    }
  }

  const formatDate = (dateString: string) =>
    new Date(dateString).toLocaleDateString('es-ES', { year: 'numeric', month: 'long', day: 'numeric' })

  return (
    <AuthenticatedLayout title={`${t('edit_user') || 'Editar Usuario'}: ${displayName}`}>
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <div className="grid flex-1 auto-rows-max gap-4">
            <div className="flex items-center gap-4">
              <Button variant="outline" size="icon" className="h-7 w-7" onClick={() => navigate(-1)}>
                <ChevronLeft className="h-4 w-4" />
                <span className="sr-only">{t('back') || 'Volver'}</span>
              </Button>
              <h1 className="flex-1 shrink-0 whitespace-nowrap text-xl font-semibold tracking-tight sm:grow-0">
                {t('edit_user') || 'Editar Usuario'}
              </h1>
              <div className="hidden items-center gap-2 md:ml-auto md:flex">
                <Button variant="outline" onClick={() => navigate(pathFor('admin.users.index'))} disabled={isSubmitting}>
                  {t('cancel') || 'Cancelar'}
                </Button>
                <Button size="sm" onClick={handleSubmit(onSubmit)} disabled={isSubmitting}>
                  {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {isSubmitting ? (t('saving') || 'Guardando...') : (t('save_changes') || 'Guardar Cambios')}
                </Button>
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-[1fr_350px] lg:gap-8">
              <div className="grid auto-rows-max items-start gap-4 lg:gap-8">
                <Card>
                  <CardHeader>
                    <CardTitle>{t('user_details') || 'Detalles'}</CardTitle>
                    <CardDescription>{t('update_user_info') || 'Actualiza los datos del usuario'}</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="grid gap-6">
                      <div className="grid gap-3">
                        <Label htmlFor="name">{t('name') || 'Nombre'}</Label>
                        <Input id="name" type="text" className="w-full" {...register('name')} />
                        {errors.name && <p className="text-sm text-destructive">{errors.name.message}</p>}
                      </div>

                      <div className="grid gap-3">
                        <Label htmlFor="username">{t('username') || 'Usuario'}</Label>
                        <Input id="username" type="text" className="w-full" {...register('username')} />
                        {errors.username && <p className="text-sm text-destructive">{errors.username.message}</p>}
                      </div>

                      <div className="grid gap-3">
                        <Label htmlFor="email">{t('email') || 'Correo'}</Label>
                        <Input id="email" type="email" className="w-full" {...register('email')} />
                        {errors.email && <p className="text-sm text-destructive">{errors.email.message}</p>}
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle>{t('change_password') || 'Cambiar Contraseña'}</CardTitle>
                    <CardDescription>
                      {t('leave_blank_keep_password') || 'Deja en blanco para mantener la actual'}
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="grid gap-6">
                      <div className="grid gap-3">
                        <Label htmlFor="password">{t('new_password') || 'Nueva Contraseña'}</Label>
                        <div className="flex gap-2">
                          <Input
                            id="password"
                            type="password"
                            className="w-full font-mono"
                            placeholder={t('generate_password_to_see_it') || 'Generar'}
                            {...register('password')}
                          />
                          <Button type="button" variant="outline" size="icon" onClick={handleGeneratePassword}>
                            <KeyRound className="h-4 w-4" />
                          </Button>
                        </div>
                        {errors.password && <p className="text-sm text-destructive">{errors.password.message}</p>}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              </div>

              <div className="grid auto-rows-max items-start gap-4 lg:gap-8">
                <Card>
                  <CardHeader>
                    <CardTitle>{t('roles') || 'Roles'}</CardTitle>
                    <CardDescription>{t('manage_user_roles') || 'Asigna un rol'}</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <Select
                      value={selectedRoleId}
                      onValueChange={handleRoleChange}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder={t('select_role') || 'Seleccionar rol'} />
                      </SelectTrigger>
                      <SelectContent>
                        {availableRoles.map((role) => (
                          <SelectItem key={role.id} value={role.id.toString()}>
                            {role.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle>{t('user_information') || 'Información'}</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-4 text-sm">
                    <div>
                      <p className="text-muted-foreground">{t('user_id') || 'ID'}</p>
                      <p className="font-medium">{user.id}</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground">{t('created_at') || 'Registrado el'}</p>
                      <p className="font-medium">{user.created_at ? formatDate(user.created_at) : '-'}</p>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </div>
          </div>
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
