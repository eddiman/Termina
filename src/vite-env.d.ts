/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_UI_ONLY?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
