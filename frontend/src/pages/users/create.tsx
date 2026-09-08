import { AuthenticatedLayout } from "@/layouts"
import { ChevronLeft, KeyRound, Loader2 } from "lucide-react"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Checkbox } from "@/components/ui/checkbox"
import { Main } from "@/components/layout"
import { useEffect, useState } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { createUserSchema, CreateUserFormValues } from "@/schemas/user.schema"
import { useNavigate } from "react-router-dom"
import { PageProps } from "@/types"
import { useI18n } from "@/i18n/context"
import { toast } from "sonner"
import { generatePassword } from "@/lib/generate-password"
import { pathFor } from "@/lib/app-routes"
import { usersApi } from "@/services/users-api"

interface Role {
  id: number
  name: string
}

interface CreateUserPageProps extends PageProps {
  roles: Role[]
}

const generateUsername = (name: string) => {
  if (!name) return ""
  return name.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase().replace(/[^a-z0-9\s]/g, '').trim().replace(/\s+/g, '.')
}

export default function CreateUser({ roles = [] }: CreateUserPageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const [usernameEdited, setUsernameEdited] = useState(false)
  const availableRoles = Array.isArray(roles) ? roles : []

  const { register, handleSubmit, watch, setValue, setError, formState: { errors, isSubmitting } } = useForm<CreateUserFormValues>({
    resolver: zodResolver(createUserSchema),
    defaultValues: { name: '', username: '', email: '', password: '', roles: [], role_ids: [] },
  })

  const name = watch('name')
  const username = watch('username')
  const role_ids = watch('role_ids') || []

  useEffect(() => {
    if (!usernameEdited && name) {
      setValue('username', generateUsername(name))
    }
  }, [name, usernameEdited, setValue])

  const handleRoleToggle = (role: Role) => {
    const currentIds = role_ids
    const currentNames = watch('roles') || []
    
    if (currentIds.includes(role.id)) {
      setValue('role_ids', currentIds.filter(id => id !== role.id))
      setValue('roles', currentNames.filter(n => n !== role.name))
    } else {
      setValue('role_ids', [...currentIds, role.id])
      setValue('roles', [...currentNames, role.name])
    }
  }

  const handleGeneratePassword = () => {
    setValue('password', generatePassword())
  }

  const onSubmit = async (values: CreateUserFormValues) => {
    try {
      await usersApi.create({
        name: values.name,
        username: values.username || generateUsername(values.name),
        email: values.email,
        password: values.password,
        role_ids: values.role_ids,
        status: 'active'
      })
      toast.success(t('user_created') || 'Usuario creado correctamente')
      navigate(pathFor('admin.users.index'))
    } catch (error: any) {
      if (error.response?.data) {
        const serverErrors = error.response.data
        Object.entries(serverErrors).forEach(([key, message]) => {
          setError(key as any, { message: Array.isArray(message) ? message[0] : message as string })
        })
      }
      toast.error(t('please_try_again') || 'Error al crear el usuario')
    }
  }

  return (
    <AuthenticatedLayout title={t('add_new_user') || 'Añadir Usuario'}>
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <div className="grid flex-1 auto-rows-max gap-4">
            <div className="flex items-center gap-4">
              <Button variant="outline" size="icon" className="h-7 w-7" onClick={() => navigate(-1)}>
                <ChevronLeft className="h-4 w-4" />
                <span className="sr-only">{t('back') || 'Volver'}</span>
              </Button>
              <h1 className="flex-1 shrink-0 whitespace-nowrap text-xl font-semibold tracking-tight sm:grow-0">
                {t('create_user') || 'Nuevo Usuario'}
              </h1>
              <div className="hidden items-center gap-2 md:ml-auto md:flex">
                <Button variant="outline" onClick={() => navigate(pathFor('admin.users.index'))} disabled={isSubmitting}>
                  {t('cancel') || 'Cancelar'}
                </Button>
                <Button size="sm" onClick={handleSubmit(onSubmit)} disabled={isSubmitting}>
                  {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {isSubmitting ? (t('creating') || 'Creando...') : (t('create_user') || 'Crear Usuario')}
                </Button>
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-[1fr_350px] lg:gap-8">
              <div className="grid auto-rows-max items-start gap-4 lg:gap-8">
                <Card>
                  <CardHeader>
                    <CardTitle>{t('user_details') || 'Detalles del Usuario'}</CardTitle>
                    <CardDescription>{t('enter_user_info') || 'Introduce la información básica del usuario'}</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="grid gap-6">
                      <div className="grid gap-3">
                        <Label htmlFor="name">{t('name') || 'Nombre Completo'}</Label>
                        <Input id="name" type="text" className="w-full" placeholder={t('enter_full_name') || 'ej. Juan Pérez'} {...register('name')} />
                        {errors.name && <p className="text-sm text-destructive">{errors.name.message}</p>}
                      </div>

                      <div className="grid gap-3">
                        <Label htmlFor="username">{t('username') || 'Nombre de Usuario'}</Label>
                        <Input
                          id="username"
                          type="text"
                          className="w-full"
                          placeholder="juan.perez"
                          {...register('username', {
                            onChange: () => setUsernameEdited(true)
                          })}
                        />
                        <p className="text-xs text-muted-foreground">{t('username_auto_generated') || 'Se genera automáticamente a partir del nombre'}</p>
                        {errors.username && <p className="text-sm text-destructive">{errors.username.message}</p>}
                      </div>

                      <div className="grid gap-3">
                        <Label htmlFor="email">{t('email') || 'Correo Electrónico'}</Label>
                        <Input id="email" type="email" className="w-full" placeholder={t('enter_email_address') || 'ejemplo@correo.com'} {...register('email')} />
                        {errors.email && <p className="text-sm text-destructive">{errors.email.message}</p>}
                      </div>

                      <div className="grid gap-3">
                        <Label htmlFor="password">{t('password') || 'Contraseña'}</Label>
                        <div className="flex gap-2">
                          <Input
                            id="password"
                            type="password"
                            className="w-full font-mono"
                            placeholder={t('generate_password_to_see_it') || 'Haz clic para generar'}
                            {...register('password')}
                          />
                          <Button type="button" variant="outline" size="icon" onClick={handleGeneratePassword} title={t('generate_password') || 'Generar contraseña'}>
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
                    <CardDescription>{t('assign_roles_to_user') || 'Asigna los roles correspondientes'}</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-3">
                      {availableRoles.map((role) => (
                        <div key={role.id} className="flex items-center space-x-2">
                          <Checkbox
                            id={`role-${role.id}`}
                            checked={role_ids.includes(role.id)}
                            onCheckedChange={() => handleRoleToggle(role)}
                          />
                          <Label htmlFor={`role-${role.id}`} className="text-sm font-normal cursor-pointer">
                            {role.name}
                          </Label>
                        </div>
                      ))}
                      {availableRoles.length === 0 && (
                        <p className="text-sm text-muted-foreground">{t('no_roles_available') || 'No hay roles disponibles'}</p>
                      )}
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle>{t('summary') || 'Resumen'}</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div>
                      <p className="text-sm text-muted-foreground">{t('name') || 'Nombre'}</p>
                      <p className="font-medium">{name || '-'}</p>
                    </div>
                    <div>
                      <p className="text-sm text-muted-foreground">{t('username') || 'Usuario'}</p>
                      <p className="font-medium">{username || generateUsername(name) || '-'}</p>
                    </div>
                    <div>
                      <p className="text-sm text-muted-foreground">{t('assigned_roles') || 'Roles asignados'}</p>
                      <p className="font-medium">{(watch('roles') || []).length > 0 ? watch('roles')?.join(', ') : (t('none') || 'Ninguno')}</p>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </div>

            <div className="flex items-center justify-center gap-2 md:hidden">
              <Button variant="outline" onClick={() => navigate(pathFor('admin.users.index'))} disabled={isSubmitting}>
                {t('cancel') || 'Cancelar'}
              </Button>
              <Button size="sm" onClick={handleSubmit(onSubmit)} disabled={isSubmitting}>
                {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                {t('create_user') || 'Crear Usuario'}
              </Button>
            </div>
          </div>
        </div>
      </Main>
    </AuthenticatedLayout>
  )
}
