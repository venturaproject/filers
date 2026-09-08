const DEFAULT_LENGTH = 15
const CHARSET = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*'

/**
 * Generate a random password.
 *
 * @param length - Password length (default: 15)
 */
export function generatePassword(length = DEFAULT_LENGTH): string {
  let password = ''
  for (let i = 0; i < length; i++) {
    password += CHARSET.charAt(Math.floor(Math.random() * CHARSET.length))
  }
  return password
}
