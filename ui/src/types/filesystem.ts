export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  mime?: string;
  modified?: number;
}

export interface FuzzyMatch {
  path: string;
  name: string;
  score: number;
}
