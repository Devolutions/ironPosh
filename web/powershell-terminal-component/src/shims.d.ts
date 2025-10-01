// Allow importing .wasm and Vite query variants like ?inline and ?url
declare module '*.wasm' {
  const url: string;
  export default url;
}
declare module '*.wasm?inline' {
  const dataUrl: string;
  export default dataUrl;
}
declare module '*.wasm?url' {
  const url: string;
  export default url;
}

// (optional but helpful with Vite)
/// <reference types="vite/client" />
