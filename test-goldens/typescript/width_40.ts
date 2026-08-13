import type { Configuration } from './config';
import type { RequestInit, ResponseBody } from './http';
import type { Logger } from './logging';

export async function handleRequest(
  config: Configuration,
  request: RequestInit, logger: Logger
): Promise<ResponseBody> {
  return undefined;
}
