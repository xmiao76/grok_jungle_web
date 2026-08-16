/* tslint:disable */
/* eslint-disable */

export function ai_move(time_budget_ms: number): string;

export function apply_move(src: string, dst: string): string;

export function engine_info(): string;

export function get_state(): string;

export function init_engine(tt_mb: number): void;

export function new_game(mode: string, human_first: boolean, difficulty: string): string;

export function replay_moves(moves_json: string): string;

export function resign(): string;

export function undo(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly ai_move: (a: number) => [number, number];
    readonly apply_move: (a: number, b: number, c: number, d: number) => [number, number];
    readonly engine_info: () => [number, number];
    readonly get_state: () => [number, number];
    readonly new_game: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly replay_moves: (a: number, b: number) => [number, number];
    readonly resign: () => [number, number];
    readonly undo: () => [number, number];
    readonly init_engine: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
