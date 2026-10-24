import { z } from "zod"

const _emptyStringToUndefined = z.literal("").transform(() => undefined)

export const asOptional = <T>(schema: z.ZodType<T>) =>
  schema.optional().or(_emptyStringToUndefined)

export const zodErrorMap: z.ZodErrorMap = (issue) => {
  const REQUIRED_ERROR_MESSAGE: string = "This field is required."
  let message = issue.message || "Invalid input."

  if (issue.code === "invalid_type" && (issue.input == null || (typeof issue.input === "number" && Number.isNaN(issue.input)))) {
    message = REQUIRED_ERROR_MESSAGE
  } else if (issue.code === "invalid_format" && issue.format === "email") {
    message = "Please input a valid email address."
  } else if (issue.code === "invalid_format" && issue.format === "url") {
    message = "Please input a valid URL."
  } else if (issue.code === "too_small" && (issue.input == null || issue.input === "")) {
    message = REQUIRED_ERROR_MESSAGE
  }

  message = message.endsWith(".") ? message : `${message}.`

  return {
    message,
  }
}

export { z }
