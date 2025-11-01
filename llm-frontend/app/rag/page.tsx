"use client"

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { PdfUpload } from "@/components/pdf-upload"
import { RagComparison } from "@/components/rag-comparison"
import { RagStatistics } from "@/components/rag-statistics"
import { FileUp, Search, BarChart3 } from "lucide-react"

export default function RagPage() {
  return (
    <div className="container mx-auto p-6 space-y-6">
      <div className="space-y-2">
        <h1 className="text-4xl font-bold tracking-tight">
          RAG Performance Comparison
        </h1>
        <p className="text-lg text-muted-foreground">
          Compare Traditional F32 embeddings vs UTP Int8 compression with semantic caching
        </p>
      </div>

      <Tabs defaultValue="upload" className="space-y-6">
        <TabsList className="grid w-full grid-cols-3 lg:w-[600px]">
          <TabsTrigger value="upload" className="flex items-center gap-2">
            <FileUp className="h-4 w-4" />
            Upload
          </TabsTrigger>
          <TabsTrigger value="search" className="flex items-center gap-2">
            <Search className="h-4 w-4" />
            Search
          </TabsTrigger>
          <TabsTrigger value="statistics" className="flex items-center gap-2">
            <BarChart3 className="h-4 w-4" />
            Statistics
          </TabsTrigger>
        </TabsList>

        <TabsContent value="upload" className="space-y-6">
          <PdfUpload
            onUploadSuccess={(result) => {
              console.log("Upload successful:", result)
            }}
          />
        </TabsContent>

        <TabsContent value="search" className="space-y-6">
          <RagComparison />
        </TabsContent>

        <TabsContent value="statistics" className="space-y-6">
          <RagStatistics />
        </TabsContent>
      </Tabs>
    </div>
  )
}
