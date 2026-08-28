/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_READ_API?: string;
  readonly VITE_WRITE_API?: string;
}

export {};

declare global {
  interface Window {
    __WB_P0_PIPELINE__?: { react: string };
    __TAURI__?: {
      event?: {
        listen?: (
          name: string,
          handler: (event: { payload?: unknown }) => void,
        ) => Promise<() => void>;
      };
      dialog?: {
        open?: (opts: {
          multiple?: boolean;
          filters?: { name: string; extensions: string[] }[];
        }) => Promise<string | string[] | null>;
      };
      core?: {
        invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        convertFileSrc?: (path: string) => string;
        Channel?: new (onmessage?: (payload: unknown) => void) => unknown;
      };
      opener?: {
        openUrl?: (url: string) => Promise<void>;
      };
    };
    __TAURI_INTERNALS__?: {
      invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    };
  }

  var marked:
    | {
        parse: (source: string) => string;
      }
    | undefined;

  var TurndownService: typeof import('turndown') | undefined;

  var QRCode:
    | {
        toCanvas: (
          canvas: HTMLCanvasElement,
          text: string,
          opts: Record<string, unknown>,
          cb: (err?: Error | null) => void,
        ) => void;
        toDataURL: (
          text: string,
          opts: Record<string, unknown>,
          cb: (err: Error | null, url?: string) => void,
        ) => void;
      }
    | undefined;

  var mermaid:
    | {
        initialize: (opts: Record<string, unknown>) => void;
        render: (
          id: string,
          source: string,
        ) => Promise<{ svg: string; bindFunctions?: (el: Element) => void }>;
      }
    | undefined;
}
