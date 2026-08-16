/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const ai_move: (a: number) => [number, number];
export const apply_move: (a: number, b: number, c: number, d: number) => [number, number];
export const engine_info: () => [number, number];
export const get_state: () => [number, number];
export const new_game: (a: number, b: number, c: number, d: number, e: number) => [number, number];
export const replay_moves: (a: number, b: number) => [number, number];
export const resign: () => [number, number];
export const undo: () => [number, number];
export const init_engine: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_start: () => void;
