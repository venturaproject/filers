import { z } from 'zod'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Switch } from '@/components/ui/switch'
import { useI18n } from '@/i18n/context'
import { toast } from 'sonner'
import { useState } from 'react'

interface NotificationSettings {
  type: 'all' | 'mentions' | 'none'
  communication_emails: boolean
  security_emails: boolean
  mobile_notifications: boolean
}

const schema = z.object({
  type: z.enum(['all', 'mentions', 'none']),
  communication_emails: z.boolean(),
  security_emails: z.boolean(),
  mobile_notifications: z.boolean(),
})

type FormValues = z.infer<typeof schema>

export function NotificationsForm() {
  const { t } = useI18n()
  const [saving, setSaving] = useState(false)

  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      type:                 'all',
      communication_emails: false,
      security_emails:      true,
      mobile_notifications: false,
    },
  })

  async function onSubmit(data: FormValues) {
    setSaving(true)
    try {
      await axios.put(endpoints.settings.notifications, data)
      toast.success(t('notifications_saved'))
    } catch {
      toast.error(t('something_went_wrong'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)} className='space-y-8'>

        {/* Tipo de notificación */}
        <FormField
          control={form.control}
          name='type'
          render={({ field }) => (
            <FormItem className='space-y-3'>
              <FormLabel>{t('notify_me_about')}</FormLabel>
              <FormControl>
                <RadioGroup
                  onValueChange={field.onChange}
                  value={field.value}
                  className='flex flex-col space-y-1'
                >
                  <FormItem className='flex items-center space-x-3 space-y-0'>
                    <FormControl><RadioGroupItem value='all' /></FormControl>
                    <FormLabel className='font-normal'>{t('all_new_messages')}</FormLabel>
                  </FormItem>
                  <FormItem className='flex items-center space-x-3 space-y-0'>
                    <FormControl><RadioGroupItem value='mentions' /></FormControl>
                    <FormLabel className='font-normal'>{t('direct_messages_and_mentions')}</FormLabel>
                  </FormItem>
                  <FormItem className='flex items-center space-x-3 space-y-0'>
                    <FormControl><RadioGroupItem value='none' /></FormControl>
                    <FormLabel className='font-normal'>{t('nothing')}</FormLabel>
                  </FormItem>
                </RadioGroup>
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        {/* Notificaciones por email */}
        <div>
          <h3 className='mb-4 text-lg font-medium'>{t('email_notifications')}</h3>
          <div className='space-y-4'>
            <FormField
              control={form.control}
              name='communication_emails'
              render={({ field }) => (
                <FormItem className='flex flex-row items-center justify-between rounded-lg border p-4'>
                  <div className='space-y-0.5'>
                    <FormLabel className='text-base'>{t('communication_emails')}</FormLabel>
                    <FormDescription>{t('communication_emails_description')}</FormDescription>
                  </div>
                  <FormControl>
                    <Switch checked={field.value} onCheckedChange={field.onChange} />
                  </FormControl>
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name='security_emails'
              render={({ field }) => (
                <FormItem className='flex flex-row items-center justify-between rounded-lg border p-4'>
                  <div className='space-y-0.5'>
                    <FormLabel className='text-base'>{t('security_emails')}</FormLabel>
                    <FormDescription>{t('security_emails_description')}</FormDescription>
                  </div>
                  <FormControl>
                    <Switch checked={field.value} onCheckedChange={field.onChange} />
                  </FormControl>
                </FormItem>
              )}
            />
          </div>
        </div>

        {/* Notificaciones móvil */}
        <FormField
          control={form.control}
          name='mobile_notifications'
          render={({ field }) => (
            <FormItem className='flex flex-row items-start space-x-3 space-y-0'>
              <FormControl>
                <Checkbox checked={field.value} onCheckedChange={field.onChange} />
              </FormControl>
              <div className='space-y-1 leading-none'>
                <FormLabel>{t('mobile_settings_label')}</FormLabel>
                <FormDescription>{t('mobile_settings_description')}</FormDescription>
              </div>
            </FormItem>
          )}
        />

        <Button type='submit' disabled={saving}>
          {saving ? t('saving') : t('update_notifications')}
        </Button>
      </form>
    </Form>
  )
}
