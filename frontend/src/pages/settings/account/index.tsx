import { SettingLayout } from '@/layouts'
import ContentSection from '../components/content-section'
import { AccountForm } from './account-form'
import { useI18n } from '@/i18n/context'

export default function SettingsAccount() {
  const { t } = useI18n()
  
  return (
    <>
      <SettingLayout title={t('account_settings')}>
        <ContentSection
          title={t('account')}
          desc={t('account_settings_description')}
        >
          <AccountForm />
        </ContentSection>
      </SettingLayout>
    </>
  )
}
