import { AxiosError, isAxiosError } from "axios"

export { AxiosError, isAxiosError }

export function isNotFound(error: unknown): boolean {
  const statusCode = isAxiosError(error)
    ? error?.response?.status
    : (error as { statusCode?: unknown })?.statusCode ?? null

  return parseInt(String(statusCode), 10) === 404
}

export function getErrorMessage(error: unknown): string {
  let message: string | undefined

  if (isAxiosError(error)) {
    message =
      error?.response?.data?.message ??
      error?.response?.data?.status?.message ??
      undefined
  } else if (error instanceof Error) {
    message = error.message
  }

  return message ?? "Server error, please try again!"
}
