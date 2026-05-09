interface FileSystemDirectoryHandle {
  name: string
  kind: 'directory'
  values(): AsyncIterableIterator<FileSystemHandle>
  entries(): AsyncIterableIterator<[string, FileSystemHandle]>
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<FileSystemDirectoryHandle>
  getFileHandle(name: string, options?: { create?: boolean }): Promise<FileSystemFileHandle>
  removeEntry(name: string, options?: { recursive?: boolean }): Promise<void>
}

interface FileSystemFileHandle {
  name: string
  kind: 'file'
  getFile(): Promise<File>
  createWritable(): Promise<FileSystemWritableFileStream>
}

interface FileSystemWritableFileStream extends WritableStream {
  write(data: FileSystemWriteChunkType): Promise<void>
  close(): Promise<void>
}

interface Window {
  showDirectoryPicker(options?: {
    mode?: 'read' | 'readwrite'
    startIn?: FileSystemHandle | string
  }): Promise<FileSystemDirectoryHandle>
}
