/// <reference types="vite/client" />

type TauriFsDirEntry = {
  name?: string;
  isDirectory?: boolean;
  isFile?: boolean;
  isSymlink?: boolean;
};

type TauriFsApi = {
  exists: (path: string) => Promise<boolean>;
  readDir: (path: string) => Promise<TauriFsDirEntry[]>;
};

interface Window {
  __TAURI__?: {
    fs?: TauriFsApi;
  };
}
