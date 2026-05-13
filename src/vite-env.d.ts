/// <reference types="vite/client" />

declare const __APP_VERSION__: string;

declare module "*.json" {
  const value: unknown;
  export default value;
}
