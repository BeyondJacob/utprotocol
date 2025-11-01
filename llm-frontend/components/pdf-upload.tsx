"use client"

import { useState, useCallback } from "react"
import { Upload, File, CheckCircle2, XCircle, Loader2 } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Progress } from "@/components/ui/progress"

interface UploadResult {
  document: {
    id: number
    title: string
    file_name: string
    file_size_bytes: number
    total_chunks: number
  }
  chunks_created: number
  f32_size_mb: number
  int8_size_mb: number
  compression_ratio: number
  processing_time_ms: number
}

interface PdfUploadProps {
  onUploadSuccess?: (result: UploadResult) => void
}

export function PdfUpload({ onUploadSuccess }: PdfUploadProps) {
  const [title, setTitle] = useState("")
  const [file, setFile] = useState<File | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [isUploading, setIsUploading] = useState(false)
  const [uploadResult, setUploadResult] = useState<UploadResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }, [])

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)

    const droppedFile = e.dataTransfer.files[0]
    if (droppedFile && droppedFile.type === "application/pdf") {
      setFile(droppedFile)
      setError(null)
    } else {
      setError("Please drop a PDF file")
    }
  }, [])

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFile = e.target.files?.[0]
    if (selectedFile) {
      if (selectedFile.type === "application/pdf") {
        setFile(selectedFile)
        setError(null)
      } else {
        setError("Please select a PDF file")
      }
    }
  }

  const handleUpload = async () => {
    if (!file || !title) {
      setError("Please provide both title and PDF file")
      return
    }

    setIsUploading(true)
    setError(null)
    setUploadResult(null)

    const formData = new FormData()
    formData.append("title", title)
    formData.append("file", file)

    try {
      const response = await fetch("http://localhost:3001/documents/upload", {
        method: "POST",
        body: formData,
      })

      if (!response.ok) {
        const errorText = await response.text()
        throw new Error(`Upload failed: ${response.statusText} - ${errorText}`)
      }

      const result: UploadResult = await response.json()
      setUploadResult(result)
      onUploadSuccess?.(result)

      // Reset form
      setTitle("")
      setFile(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Upload failed")
    } finally {
      setIsUploading(false)
    }
  }

  const formatFileSize = (bytes: number) => {
    return (bytes / 1024 / 1024).toFixed(2) + " MB"
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle>Upload PDF Document</CardTitle>
        <CardDescription>
          Upload a PDF to create dual embeddings (F32 + Int8) for RAG comparison
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Title Input */}
        <div className="space-y-2">
          <Label htmlFor="title">Document Title</Label>
          <Input
            id="title"
            placeholder="e.g., UTP Protocol Documentation"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            disabled={isUploading}
          />
        </div>

        {/* Drag & Drop Zone */}
        <div
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          className={`
            border-2 border-dashed rounded-lg p-8 text-center cursor-pointer
            transition-colors duration-200
            ${isDragging ? "border-primary bg-primary/5" : "border-muted-foreground/25"}
            ${file ? "bg-muted/50" : ""}
          `}
        >
          {file ? (
            <div className="flex items-center justify-center gap-2">
              <File className="h-5 w-5 text-primary" />
              <span className="text-sm font-medium">{file.name}</span>
              <span className="text-xs text-muted-foreground">
                ({formatFileSize(file.size)})
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setFile(null)}
                disabled={isUploading}
              >
                <XCircle className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              <Upload className="h-8 w-8 mx-auto text-muted-foreground" />
              <p className="text-sm text-muted-foreground">
                Drag and drop a PDF file here, or click to browse
              </p>
              <input
                type="file"
                accept=".pdf"
                onChange={handleFileChange}
                className="hidden"
                id="file-input"
                disabled={isUploading}
              />
              <label htmlFor="file-input">
                <Button variant="outline" size="sm" asChild disabled={isUploading}>
                  <span>Browse Files</span>
                </Button>
              </label>
            </div>
          )}
        </div>

        {/* Upload Button */}
        <Button
          onClick={handleUpload}
          disabled={!file || !title || isUploading}
          className="w-full"
        >
          {isUploading ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Processing...
            </>
          ) : (
            <>
              <Upload className="mr-2 h-4 w-4" />
              Upload and Process
            </>
          )}
        </Button>

        {/* Error Display */}
        {error && (
          <div className="flex items-center gap-2 p-3 rounded-md bg-destructive/10 text-destructive">
            <XCircle className="h-4 w-4" />
            <span className="text-sm">{error}</span>
          </div>
        )}

        {/* Success Result */}
        {uploadResult && (
          <div className="space-y-3 p-4 rounded-md bg-green-50 dark:bg-green-950/20 border border-green-200 dark:border-green-900">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />
              <span className="font-medium text-green-900 dark:text-green-100">
                Upload Successful!
              </span>
            </div>

            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-muted-foreground">Chunks Created:</span>
                <span className="ml-2 font-medium">{uploadResult.chunks_created}</span>
              </div>
              <div>
                <span className="text-muted-foreground">Processing Time:</span>
                <span className="ml-2 font-medium">{uploadResult.processing_time_ms}ms</span>
              </div>
              <div>
                <span className="text-muted-foreground">F32 Size:</span>
                <span className="ml-2 font-medium">{uploadResult.f32_size_mb.toFixed(3)} MB</span>
              </div>
              <div>
                <span className="text-muted-foreground">Int8 Size:</span>
                <span className="ml-2 font-medium">{uploadResult.int8_size_mb.toFixed(3)} MB</span>
              </div>
              <div className="col-span-2">
                <span className="text-muted-foreground">Compression Ratio:</span>
                <span className="ml-2 font-medium text-primary">
                  {uploadResult.compression_ratio.toFixed(2)}x
                </span>
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
