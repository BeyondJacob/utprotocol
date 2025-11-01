"use client"

import { useEffect, useState } from "react"
import { RefreshCw, FileText, Database, Zap, TrendingUp, Loader2 } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"

interface DocumentInfo {
  id: number
  title: string
  total_chunks: number
}

interface VectorStatistics {
  id: number
  total_documents: number
  total_chunks: number
  total_f32_size_mb: number
  total_int8_size_mb: number
  avg_compression_ratio: number
  storage_savings_percent: number
  total_queries: number
  avg_traditional_latency_ms: number
  avg_utp_latency_ms: number
  avg_speedup_factor: number
  utp_cache_hit_rate: number
  avg_retrieval_overlap: number
  last_updated: string
  documents: DocumentInfo[]
}

export function RagStatistics() {
  const [stats, setStats] = useState<VectorStatistics | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchStatistics = async () => {
    setIsLoading(true)
    setError(null)

    try {
      const response = await fetch("http://localhost:3001/rag/statistics")

      if (!response.ok) {
        throw new Error(`Failed to fetch statistics: ${response.statusText}`)
      }

      const data: VectorStatistics = await response.json()
      setStats(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load statistics")
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    fetchStatistics()
  }, [])

  if (isLoading && !stats) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center p-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center space-y-3">
            <p className="text-destructive">{error}</p>
            <Button onClick={fetchStatistics} variant="outline" size="sm">
              <RefreshCw className="mr-2 h-4 w-4" />
              Retry
            </Button>
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!stats) {
    return null
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">RAG Statistics</h2>
          <p className="text-muted-foreground">
            Real-time performance metrics for Traditional vs UTP comparison
          </p>
        </div>
        <Button onClick={fetchStatistics} variant="outline" size="sm" disabled={isLoading}>
          {isLoading ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="mr-2 h-4 w-4" />
          )}
          Refresh
        </Button>
      </div>

      {/* Document & Storage Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
              <FileText className="h-4 w-4" />
              Total Documents
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold">{stats.total_documents}</div>
            <p className="text-xs text-muted-foreground mt-1">
              {stats.total_chunks} chunks
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
              <Database className="h-4 w-4" />
              Compression Ratio
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-primary">
              {stats.avg_compression_ratio.toFixed(2)}x
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              Int8 vs F32
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
              <Zap className="h-4 w-4" />
              Avg Speedup
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-yellow-600 dark:text-yellow-400">
              {stats.avg_speedup_factor > 0 ? stats.avg_speedup_factor.toFixed(2) : "N/A"}x
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {stats.total_queries} queries
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-muted-foreground">
              <TrendingUp className="h-4 w-4" />
              Cache Hit Rate
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-bold text-green-600 dark:text-green-400">
              {stats.utp_cache_hit_rate.toFixed(1)}%
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              Semantic caching
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Storage Metrics */}
      <Card>
        <CardHeader>
          <CardTitle>Storage Comparison</CardTitle>
          <CardDescription>
            Embedding storage usage across all documents
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Traditional F32</span>
              <span className="font-medium">{stats.total_f32_size_mb.toFixed(3)} MB</span>
            </div>
            <Progress value={100} className="h-2" />

            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">UTP Int8</span>
              <span className="font-medium text-primary">{stats.total_int8_size_mb.toFixed(3)} MB</span>
            </div>
            <Progress
              value={(stats.total_int8_size_mb / stats.total_f32_size_mb) * 100}
              className="h-2"
            />
          </div>

          <div className="p-4 rounded-lg bg-primary/5 border border-primary/20">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Storage Savings</span>
              <span className="text-2xl font-bold text-primary">
                {stats.storage_savings_percent.toFixed(1)}%
              </span>
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              Saved {(stats.total_f32_size_mb - stats.total_int8_size_mb).toFixed(3)} MB
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Query Performance */}
      {stats.total_queries > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Query Performance</CardTitle>
            <CardDescription>
              Average latency across all RAG queries
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Traditional</p>
                <p className="text-2xl font-bold">{stats.avg_traditional_latency_ms.toFixed(0)}ms</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">UTP</p>
                <p className="text-2xl font-bold text-primary">{stats.avg_utp_latency_ms.toFixed(0)}ms</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Retrieval Overlap</p>
                <p className="text-2xl font-bold">{(stats.avg_retrieval_overlap * 100).toFixed(1)}%</p>
              </div>
            </div>

            <div className="p-3 rounded-md bg-muted/50 text-xs text-muted-foreground">
              Last updated: {new Date(stats.last_updated).toLocaleString()}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Uploaded Documents List */}
      {stats.documents && stats.documents.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Uploaded Documents</CardTitle>
            <CardDescription>
              {stats.documents.length} document{stats.documents.length !== 1 ? 's' : ''} with {stats.total_chunks} total chunks
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {stats.documents.map((doc) => (
                <div
                  key={doc.id}
                  className="flex items-center justify-between p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <FileText className="h-5 w-5 text-muted-foreground" />
                    <div>
                      <p className="font-medium">{doc.title}</p>
                      <p className="text-sm text-muted-foreground">
                        {doc.total_chunks} chunk{doc.total_chunks !== 1 ? 's' : ''}
                      </p>
                    </div>
                  </div>
                  <div className="text-sm text-muted-foreground">ID: {doc.id}</div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* No Data Message */}
      {stats.total_documents === 0 && (
        <Card>
          <CardContent className="p-12">
            <div className="text-center space-y-3">
              <Database className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <h3 className="font-medium text-lg">No documents uploaded yet</h3>
                <p className="text-sm text-muted-foreground">
                  Upload a PDF document to start seeing statistics
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
