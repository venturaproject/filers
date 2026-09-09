import { env } from "@/config"

export function assetUrl(path: string | undefined) {
  if (!path) return ""
  if (path.startsWith("http")) return path

  if (path.startsWith("/")) {
    path = path.slice(1)
  }

  return `${env.ASSET_URL}/${path}`
}

export function appUrl(
  path: string | Record<string, unknown> = "",
  params: Record<string, unknown> | undefined = undefined
) {
  if (!params && typeof path === "object") {
    params = path
    path = ""
  }

  let pathStr = typeof path === "string" ? path : ""

  if (params) {
    const query = new URLSearchParams(
      Object.entries(params).map(([k, v]) => [k, String(v)])
    )
    pathStr = `${pathStr}?${query.toString()}`
  }

  if (pathStr.startsWith("/")) pathStr = pathStr.slice(1)

  return `${env.APP_URL}/${pathStr}`.replace(/\/$/, "")
}
