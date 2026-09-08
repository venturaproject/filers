import { z } from 'zod'
import { useEffect, useRef } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { PasswordInput } from '@/components/password-input'
import { SelectDropdown } from '@/components/select-dropdown'
import { User } from '../data/schema'
import { useI18n } from '@/i18n/context'
import { rolesApi } from '@/services/roles-api'
import { usersApi } from '@/services/users-api'

interface Props {
  currentRow?: User
  open: boolean
  onOpenChange: (open: boolean) => void
}

const generateUsername = (first: string, last: string) => {
  const firstName  = first.trim().split(/\s+/)[0] ?? ''
  const firstLast  = last.trim().split(/\s+/)[0] ?? ''
  return `${firstName} ${firstLast}`
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, '')
    .trim()
    .replace(/\s+/g, '.')
}

export function UsersActionDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const isEdit = !!currentRow

  const { data: rolesData } = useQuery({
    queryKey: ['roles', 'all'],
    queryFn: () => rolesApi.all({ per_page: 1000 }),
    enabled: open,
  })
  const roleItems = ((rolesData ?? []) as Array<{ id: number; name: string }>).map((r) => ({
    label: r.name,
    value: r.name,
  }))

  const formSchema = z
    .object({
      firstName:       z.string().min(1, { message: t('first_name_required') }),
      lastName:        z.string().min(1, { message: t('last_name_required') }),
      username:        z.string(),
      email:           z.string().min(1, { message: t('email_required') }).email({ message: t('email_invalid') }),
      role:            z.string().min(1, { message: t('role_required') }),
      password:        z.string().transform((pwd) => pwd.trim()),
      confirmPassword: z.string().transform((pwd) => pwd.trim()),
      isEdit:          z.boolean(),
    })
    .superRefine(({ isEdit, password, confirmPassword }, ctx) => {
      if (!isEdit || (isEdit && password !== '')) {
        if (password === '') {
          ctx.addIssue({ code: z.ZodIssueCode.custom, message: t('password_required'), path: ['password'] })
        }
        if (password.length < 8) {
          ctx.addIssue({ code: z.ZodIssueCode.custom, message: t('password_min_length'), path: ['password'] })
        }
        if (!password.match(/[a-z]/)) {
          ctx.addIssue({ code: z.ZodIssueCode.custom, message: t('password_lowercase'), path: ['password'] })
        }
        if (!password.match(/\d/)) {
          ctx.addIssue({ code: z.ZodIssueCode.custom, message: t('password_number'), path: ['password'] })
        }
        if (password !== confirmPassword) {
          ctx.addIssue({ code: z.ZodIssueCode.custom, message: t('passwords_dont_match'), path: ['confirmPassword'] })
        }
      }
    })

  type UserForm = z.infer<typeof formSchema>

  const form = useForm<UserForm>({
    resolver: zodResolver(formSchema),
    defaultValues: isEdit
      ? {
          firstName:       currentRow.firstName,
          lastName:        currentRow.lastName,
          username:        currentRow.username ?? '',
          email:           currentRow.email,
          role:            currentRow.role ?? '',
          password:        '',
          confirmPassword: '',
          isEdit:          true,
        }
      : {
          firstName:       '',
          lastName:        '',
          username:        '',
          email:           '',
          role:            '',
          password:        '',
          confirmPassword: '',
          isEdit:          false,
        },
  })

  const usernameManuallyEdited = useRef(isEdit)
  const firstName = useWatch({ control: form.control, name: 'firstName' })
  const lastName  = useWatch({ control: form.control, name: 'lastName' })

  useEffect(() => {
    if (!usernameManuallyEdited.current) {
      form.setValue('username', generateUsername(firstName, lastName), { shouldValidate: false })
    }
  }, [firstName, lastName])

  const onSubmit = async (values: UserForm) => {
    const payload: Record<string, string | string[]> = {
      name:     `${values.firstName} ${values.lastName}`.trim(),
      username: values.username || generateUsername(values.firstName, values.lastName),
      email:    values.email,
      roles:    values.role ? [values.role] : [],
    }

    if (values.password) {
      payload.password              = values.password
      payload.password_confirmation = values.confirmPassword
    }

    const handleErrors = (errors: Record<string, string>) => {
      Object.entries(errors).forEach(([field, message]) => {
        const map: Record<string, keyof UserForm> = {
          name:     'firstName',
          username: 'username',
          email:    'email',
          password: 'password',
          roles:    'role',
        }
        const formField = map[field] as keyof UserForm | undefined
        if (formField) form.setError(formField, { message })
      })
      toast.error(t('please_check_form_and_try_again'))
    }

    try {
      if (isEdit) {
        await usersApi.update(currentRow!.id, payload)
        toast.success(t('user_updated'))
      } else {
        await usersApi.create(payload)
        toast.success(t('user_created'))
      }
      queryClient.invalidateQueries({ queryKey: ['users'] })
      form.reset()
      usernameManuallyEdited.current = isEdit
      onOpenChange(false)
    } catch (err: any) {
      const errors = err.response?.data ?? {}
      if (Object.keys(errors).length > 0) {
        handleErrors(errors)
      } else {
        toast.error(t('please_try_again'))
      }
    }
  }

  const isPasswordTouched = !!form.formState.dirtyFields.password

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        form.reset()
        usernameManuallyEdited.current = isEdit
        onOpenChange(state)
      }}
    >
      <DialogContent className='sm:max-w-2xl'>
        <DialogHeader className='text-left'>
          <DialogTitle>{isEdit ? t('edit_user') : t('add_new_user')}</DialogTitle>
          <DialogDescription>
            {isEdit ? t('update_user_description') : t('create_user_description')}
            {' '}{t('click_save_when_done')}
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className='h-[32rem] w-full pr-4 -mr-4 py-1'>
          <Form {...form}>
            <form id='user-form' onSubmit={form.handleSubmit(onSubmit)} className='space-y-4 p-0.5' autoComplete='off'>
              <input type='text' name='fake_user' style={{ display: 'none' }} readOnly />
              <input type='password' name='fake_pass' style={{ display: 'none' }} readOnly />
              <FormField
                control={form.control}
                name='firstName'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('first_name')}</FormLabel>
                    <FormControl>
                      <Input placeholder='Antonio' className='col-span-4' autoComplete='off' {...field} />
                    </FormControl>
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='lastName'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('last_name')}</FormLabel>
                    <FormControl>
                      <Input placeholder='Ventura' className='col-span-4' autoComplete='off' {...field} />
                    </FormControl>
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='username'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('username')}</FormLabel>
                    <FormControl>
                      <Input
                        placeholder='antonio.ventura'
                        className='col-span-4'
                        autoComplete='new-password'
                        {...field}
                        onChange={(e) => {
                          usernameManuallyEdited.current = true
                          field.onChange(e)
                        }}
                      />
                    </FormControl>
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='email'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('email')}</FormLabel>
                    <FormControl>
                      <Input placeholder='antonio@ejemplo.com' className='col-span-4' {...field} />
                    </FormControl>
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='role'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('role')}</FormLabel>
                    <SelectDropdown
                      defaultValue={field.value}
                      onValueChange={field.onChange}
                      placeholder={t('select_role')}
                      className='col-span-4'
                      items={roleItems}
                    />
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='password'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('password')}</FormLabel>
                    <FormControl>
                      <PasswordInput placeholder='e.g., S3cur3P@ssw0rd' className='col-span-4' {...field} />
                    </FormControl>
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='confirmPassword'
                render={({ field }) => (
                  <FormItem className='grid grid-cols-6 items-center gap-x-4 gap-y-1 space-y-0'>
                    <FormLabel className='col-span-2 text-right'>{t('confirm_password')}</FormLabel>
                    <FormControl>
                      <PasswordInput
                        disabled={!isPasswordTouched}
                        placeholder='e.g., S3cur3P@ssw0rd'
                        className='col-span-4'
                        {...field}
                      />
                    </FormControl>
                    <FormMessage className='col-span-4 col-start-3' />
                  </FormItem>
                )}
              />
            </form>
          </Form>
        </ScrollArea>
        <DialogFooter>
          <Button type='submit' form='user-form'>
            {t('save_changes')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
