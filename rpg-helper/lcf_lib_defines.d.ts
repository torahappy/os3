import type {MainModule} from "./dist-wasm/rpg_lsd_io";

declare global {
    interface Window { call_lcf_lib_data: CallLcfLibData; call_lcf_lib: CallLcfLibFunc; }
}

export type LcfMessage = LcfMessageRead | LcfMessageWrite | LcfMessageWriteSwitches | LcfMessageWriteFile | LcfMessageReadFile
export type LcfMessageReturn = null | Boolean[] | Int32Array | Uint8Array

export interface LcfResolve {
  (data: LcfMessage): void
}

export interface LcfReject {
  (error: Error): void
}

export interface LcfResolveReject {
  resolve: LcfResolve,
  reject: LcfReject
}

export interface CallLcfLibFunc {
  (function_name: string, args: LcfMessage): Promise<LcfMessage>
}

export interface CallLcfLibData {
  worker: Worker | null,
  pending: Map<number, LcfResolveReject>,
  lastId: number
}

export interface LcfMessageRead {
  filename: string,
  offset: number,
  count: number
}

export interface LcfMessageWrite {
  in_filename: string,
  out_filename: string,
  offset: number,
  count: number,
  variables: number[]
}

export interface LcfMessageWriteSwitches {
  in_filename: string,
  out_filename: string,
  offset: number,
  count: number,
  switches: number[]
}
export interface LcfMessageWriteFile {
  filename: string,
  data: Uint8Array
}

export interface LcfMessageReadFile {
  filename: string
}

export interface MallocFunc {
  (size: number): number
}

export interface FreeFunc {
  (address: number): void
}

export interface ReadCallFunc {
  (ptrName: number, offset: number, count: number, retPtr: number): number
}

export interface WriteCallFunc {
  (ptrIn: number, ptrOut: number, offset: number, count: number, ptrVar: number): number
}

export interface FSDef {
  readFile(filename: string): Uint8Array,
  writeFile(filename: string, data: Uint8Array): void
}

export interface MainModuleWithFS extends MainModule {
  FS: FSDef;
}
