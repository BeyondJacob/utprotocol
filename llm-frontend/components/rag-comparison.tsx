"use client"

import { useState } from "react"
import { Search, Zap, Database, TrendingUp, Loader2 } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"

interface RetrievedChunk {
  chunk_id: number
  document_id: number
  document_title: string
  chunk_index: number
  content: string
  similarity_score: number
  rank: number
}

interface RagQueryResponse {
  query: string
  chunks: RetrievedChunk[]
  latency_ms: number
  total_size_bytes: number
  approach: string
  cache_hit: boolean | null
  similarity_score: number | null
}

interface ComparisonMetrics {
  speedup_factor: number
  size_reduction_percent: number
  retrieval_overlap_percent: number
  chunks_in_common: number
  traditional_only: number
  utp_only: number
}

interface RagComparisonResult {
  query: string
  traditional: RagQueryResponse
  utp: RagQueryResponse
  comparison: ComparisonMetrics
}

export function RagComparison() {
  const [query, setQuery] = useState("")
  const [topK, setTopK] = useState(3)
  const [isSearching, setIsSearching] = useState(false)
  const [result, setResult] = useState<RagComparisonResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  const handleSearch = async () => {
    if (!query.trim()) {
      setError("Please enter a search query")
      return
    }

    setIsSearching(true)
    setError(null)
    setResult(null)

    try {
      const response = await fetch("http://localhost:3001/rag/query", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          query: query.trim(),
          top_k: topK,
          use_utp: true,
        }),
      })

      if (!response.ok) {
        throw new Error(`Search failed: ${response.statusText}`)
      }

      const data: RagComparisonResult = await response.json()
      setResult(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Search failed")
    } finally {
      setIsSearching(false)
    }
  }

  const formatBytes = (bytes: number) => {
    return (bytes / 1024).toFixed(2) + " KB"
  }

  return (
    <div className="space-y-6">
      {/* Search Input */}
      <Card>
        <CardHeader>
          <CardTitle>RAG Query Comparison</CardTitle>
          <CardDescription>
            Compare Traditional F32 vs UTP Int8 embeddings with semantic caching
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Input
              placeholder="Enter your query... (e.g., What is UTP protocol?)"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              disabled={isSearching}
              className="flex-1"
            />
            <Input
              type="number"
              min={1}
              max={10}
              value={topK}
              onChange={(e) => setTopK(parseInt(e.target.value) || 3)}
              disabled={isSearching}
              className="w-20"
              title="Top K results"
            />
            <Button onClick={handleSearch} disabled={isSearching || !query.trim()}>
              {isSearching ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Searching...
                </>
              ) : (
                <>
                  <Search className="mr-2 h-4 w-4" />
                  Search
                </>
              )}
            </Button>
          </div>

          {error && (
            <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
              {error}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Comparison Results */}
      {result && (
        <>
          {/* Metrics Overview */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium flex items-center gap-2">
                  <Zap className="h-4 w-4 text-yellow-500" />
                  Speedup Factor
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-primary">
                  {result.comparison.speedup_factor.toFixed(2)}x
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  UTP is {result.comparison.speedup_factor > 1 ? "faster" : "slower"}
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium flex items-center gap-2">
                  <Database className="h-4 w-4 text-blue-500" />
                  Storage Reduction
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-primary">
                  {result.comparison.size_reduction_percent.toFixed(1)}%
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {formatBytes(result.traditional.total_size_bytes)} →{" "}
                  {formatBytes(result.utp.total_size_bytes)}
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium flex items-center gap-2">
                  <TrendingUp className="h-4 w-4 text-green-500" />
                  Retrieval Overlap
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-primary">
                  {result.comparison.retrieval_overlap_percent.toFixed(1)}%
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {result.comparison.chunks_in_common} chunks in common
                </p>
              </CardContent>
            </Card>
          </div>

          {/* Side-by-Side Results */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {/* Traditional Results */}
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="text-lg">Traditional (F32)</CardTitle>
                  <Badge variant="secondary">
                    {result.traditional.latency_ms}ms
                  </Badge>
                </div>
                <CardDescription>
                  Full precision, no caching | {formatBytes(result.traditional.total_size_bytes)}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {result.traditional.chunks.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No results found</p>
                ) : (
                  result.traditional.chunks.map((chunk) => (
                    <div
                      key={`trad-${chunk.chunk_id}`}
                      className="p-3 rounded-md border bg-card space-y-2"
                    >
                      <div className="flex items-center justify-between">
                        <Badge variant="outline" className="text-xs">
                          Rank #{chunk.rank}
                        </Badge>
                        <span className="text-xs text-muted-foreground">
                          Score: {chunk.similarity_score.toFixed(4)}
                        </span>
                      </div>
                      <p className="text-sm leading-relaxed">{chunk.content}</p>
                      <div className="text-xs text-muted-foreground">
                        Document ID: {chunk.document_id} | Chunk: {chunk.chunk_index}
                      </div>
                    </div>
                  ))
                )}
              </CardContent>
            </Card>

            {/* UTP Results */}
            <Card className="border-primary/50">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="text-lg flex items-center gap-2">
                    UTP (Int8)
                    {result.utp.cache_hit && (
                      <Badge variant="default" className="text-xs">
                        Cache Hit
                      </Badge>
                    )}
                  </CardTitle>
                  <Badge variant="default">
                    {result.utp.latency_ms}ms
                  </Badge>
                </div>
                <CardDescription>
                  Compressed embeddings + semantic cache | {formatBytes(result.utp.total_size_bytes)}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {result.utp.chunks.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No results found</p>
                ) : (
                  result.utp.chunks.map((chunk) => {
                    const inBoth = result.traditional.chunks.some(
                      (tc) => tc.chunk_id === chunk.chunk_id
                    )
                    return (
                      <div
                        key={`utp-${chunk.chunk_id}`}
                        className={`p-3 rounded-md border space-y-2 ${
                          inBoth ? "bg-primary/5 border-primary/20" : "bg-card"
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <Badge variant={inBoth ? "default" : "outline"} className="text-xs">
                            Rank #{chunk.rank} {inBoth && "✓"}
                          </Badge>
                          <span className="text-xs text-muted-foreground">
                            Score: {chunk.similarity_score.toFixed(4)}
                          </span>
                        </div>
                        <p className="text-sm leading-relaxed">{chunk.content}</p>
                        <div className="text-xs text-muted-foreground">
                          Document ID: {chunk.document_id} | Chunk: {chunk.chunk_index}
                        </div>
                      </div>
                    )
                  })
                )}
              </CardContent>
            </Card>
          </div>
        </>
      )}
    </div>
  )
}
