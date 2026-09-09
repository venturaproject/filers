import axiosRequest from "axios"
import { axios } from "@/lib/axios"
import { endpoints } from "@/lib/endpoints"
import { assetUrl } from "@/lib/urls"

interface UploaderOptions {
  unique?: boolean
  bucketId?: string
  onUploadProgress?: (event: unknown) => void
}

function createImageUploader({
  unique = true,
  bucketId = "uploads/images",
  onUploadProgress,
}: UploaderOptions = {}) {
  return async (file: File): Promise<string> => {
    const response = await axios.request({
      method: "POST",
      url: endpoints.upload.presignedUrl,
      data: {
        unique,
        bucketId,
        filename: file.name,
        mimeType: file.type,
        contentLength: file.size,
      },
    })

    const url: string | undefined = response?.data?.url
    const path: string | undefined = response?.data?.path ?? response?.data?.key
    const headers: Record<string, string> | undefined = response?.data?.headers

    await axiosRequest.request({
      method: "PUT",
      url: url!,
      data: file,
      onUploadProgress,
      headers: {
        "Content-Type": file.type,
        "Content-Disposition": `inline; filename="${file.name}"`,
        ...headers,
      },
    })

    return assetUrl(path ?? url)
  }
}

export { createImageUploader }
