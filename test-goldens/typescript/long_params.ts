import type { Configuration } from './config';
import type { RequestInit } from './http';
import type { InitOverrideFunction } from './runtime';

export async function createUser( name: string, age: number,
  config: Configuration, request: RequestInit, override: InitOverrideFunction
): Promise<void> {
  return undefined;
}
