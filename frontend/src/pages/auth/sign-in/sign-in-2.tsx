const TradivelLogo = '/logo-tradivel-compact-light.svg'
import { DocumentTitle } from '@/components/document-title'
import { UserAuthForm } from './components/user-auth-form'
import { useI18n } from '@/i18n/context'
import { APP_NAME } from '@/config/env'

export default function SignIn2({
    status,
    canResetPassword,
  }: {
  status?: string;
  canResetPassword: boolean;
}) {
  const { t } = useI18n()
  return (
    <>
      <DocumentTitle title={t('login')}/>
      <div
        className='container relative grid h-svh flex-col items-center justify-center lg:max-w-none lg:grid-cols-2 lg:px-0'>
        <div className='relative hidden h-full flex-col bg-muted p-10 text-white dark:border-r lg:flex'>
          <div className='absolute inset-0 bg-zinc-900'/>
          <div className='relative z-20 flex items-center text-lg font-medium'>
     
           
          </div>

          <img
            src={TradivelLogo}
            className='relative m-auto'
            width={301}
            height={60}
            alt='Ocrer'
          />

          <div className='relative z-20 mt-auto'>
            <blockquote className='space-y-2'>
              <footer className='text-sm text-center'>
                {APP_NAME} © {new Date().getFullYear()}
              </footer>
            </blockquote>
          </div>
        </div>
        <div className='lg:p-8'>
          <div className='mx-auto flex w-full flex-col justify-center space-y-2 sm:w-[350px]'>
            <div className='flex flex-col space-y-2 text-left'>
              <h1 className='text-2xl font-semibold tracking-tight'>Iniciar sesión</h1>
              <p className='text-sm text-muted-foreground'>
                {t('enter_email_and_password_below')} <br/>
                {t('to_log_into_your_account')}
                {/* , do not have an account?
                <Link
                  className='underline underline-offset-4 hover:text-primary'
                  href={route('register')}>Register</Link> */}
              </p>
            </div>
            <UserAuthForm canResetPassword={false} status={status} />
            {/* <TermPrivacyLink privacyLink={'#'} termLink={'#'} /> */}
            {/* <SocialButtons isLoading={false}/> */}
          </div>
        </div>
      </div>
    </>
  )
}
