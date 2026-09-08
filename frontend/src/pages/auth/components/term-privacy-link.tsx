import {Link} from "react-router-dom";

export function TermPrivacyLink({termLink='', privacyLink = ''}) {
  return (
    <>
      <p className='mt-4 px-8 text-center text-sm text-muted-foreground'>
        By clicking login, you agree to our{' '}
        <Link
          to={termLink}
          className='underline underline-offset-4 hover:text-primary'
        >
          Terms of Service
        </Link>{' '}
        and{' '}
        <Link
          to={privacyLink}
          className='underline underline-offset-4 hover:text-primary'
        >
          Privacy Policy
        </Link>
        .
      </p>
    </>
  )
}
