'use client'

import { useState } from 'react'
import { IconAlertTriangle } from '@tabler/icons-react'
import { toast } from 'sonner'
import { useQueryClient } from '@tanstack/react-query'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { User } from '../data/schema'
import { useI18n } from '@/i18n/context'
import { usersApi } from '@/services/users-api'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: User
}

export function UsersDeleteDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const [value, setValue] = useState('')

  const handleDelete = async () => {
    if (value.trim() !== currentRow.username) return

    try {
      await usersApi.delete(currentRow.id)
      queryClient.invalidateQueries({ queryKey: ['users'] })
      toast.success(t('user_deleted_successfully') || 'Usuario eliminado correctamente')
      onOpenChange(false)
      setValue('')
    } catch {
      toast.error(t('something_went_wrong') || 'Error al eliminar el usuario')
    }
  }

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(state) => {
        if (!state) setValue('')
        onOpenChange(state)
      }}
      handleConfirm={handleDelete}
      disabled={value.trim() !== currentRow.username}
      title={
        <span className='text-destructive'>
          <IconAlertTriangle
            className='mr-1 inline-block stroke-destructive'
            size={18}
          />{' '}
          {t('delete_user') || 'Eliminar Usuario'}
        </span>
      }
      desc={
        <div className='space-y-4'>
          <p className='mb-2'>
            {t('are_you_sure_delete_user') || '¿Estás seguro de que deseas eliminar'}{' '}
            <span className='font-bold'>{currentRow.username}</span>?
            <br />
            {t('user_will_be_permanently_removed') || 'Esta acción eliminará permanentemente al usuario con el rol'}{' '}
            <span className='font-bold'>
              {currentRow.role?.toUpperCase()}
            </span>{' '}
            {t('this_cannot_be_undone') || 'del sistema. Esta acción no se puede deshacer.'}
          </p>

          <Label className='my-2'>
            {t('username') || 'Nombre de usuario'}:
            <Input
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder={t('enter_username_to_confirm') || 'Escribe el nombre de usuario para confirmar'}
            />
          </Label>

          <Alert variant='destructive'>
            <AlertTitle>{t('warning') || 'Advertencia'}</AlertTitle>
            <AlertDescription>
              {t('please_be_careful') || 'Ten cuidado, esta operación no se puede revertir.'}
            </AlertDescription>
          </Alert>
        </div>
      }
      confirmText={t('delete') || 'Eliminar'}
      destructive
    />
  )
}
