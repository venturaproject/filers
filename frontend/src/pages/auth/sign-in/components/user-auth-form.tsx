import { HTMLAttributes } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { Link, useNavigate } from 'react-router-dom'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PasswordInput } from '@/components/password-input'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import InputError from '@/components/InputError'
import { getAuthAdapter } from '@/lib/auth-adapter'
import { useAuthStore } from '@/lib/auth'

const schema = z.object({
  login: z.string().min(1, 'El email o usuario es requerido'),
  password: z.string().min(1, 'La contraseña es requerida'),
  remember: z.boolean(),
})
type FormData = z.infer<typeof schema>

type UserAuthFormProps = HTMLAttributes<HTMLDivElement> & {
  status?: string
  canResetPassword?: boolean
}

export function UserAuthForm({ className, status, canResetPassword = true, ...props }: UserAuthFormProps) {
  const navigate = useNavigate()
  const setAuth = useAuthStore((s) => s.setAuth)

  const { register, handleSubmit, formState: { errors, isSubmitting }, setError, watch, setValue } =
    useForm<FormData>({ resolver: zodResolver(schema), defaultValues: { login: '', password: '', remember: false } })

  const onSubmit = async (data: FormData) => {
    try {
      const session = await getAuthAdapter().login(data)
      setAuth(session.user, session.permissions, session.roles)
      navigate('/admin')
    } catch (err: any) {
      const message = err.response?.data?.message ?? 'Credenciales incorrectas'
      setError('login', { message })
    }
  }

  return (
    <div className={cn('grid gap-6', className)} {...props}>
      {status && <div className="text-sm font-medium text-green-600">{status}</div>}

      <form onSubmit={handleSubmit(onSubmit)} noValidate>
        <div className='grid gap-2'>
          <div className='space-y-1'>
            <Label htmlFor="login">Email o usuario</Label>
            <Input
              id="login"
              type="text"
              placeholder='nombre.apellido o email@ejemplo.com'
              autoComplete="username"
              {...register('login')}
            />
            <InputError message={errors.login?.message} className="mt-2" />
          </div>

          <div className='space-y-1'>
            <div className='flex items-center justify-between'>
              <Label htmlFor="password">Password</Label>
              {canResetPassword && (
                <Link
                  to='/forgot-password'
                  className='text-sm font-medium text-muted-foreground hover:opacity-75'
                  tabIndex={1}
                >
                  Forgot password?
                </Link>
              )}
            </div>
            <PasswordInput
              id="password"
              placeholder='********'
              autoComplete="current-password"
              {...register('password')}
            />
            <InputError message={errors.password?.message} className="mt-2" />
          </div>

          <div className="flex flex-row items-center space-x-2 space-y-0 mt-2">
            <Checkbox
              id="remember"
              checked={watch('remember')}
              onCheckedChange={(checked) => setValue('remember', !!checked)}
            />
            <label htmlFor="remember" className="text-sm font-normal text-muted-foreground">
              Recordarme
            </label>
          </div>

          <Button className='mt-4' disabled={isSubmitting}>
            {isSubmitting ? 'Accediendo...' : 'Entrar'}
          </Button>
        </div>
      </form>
    </div>
  )
}
