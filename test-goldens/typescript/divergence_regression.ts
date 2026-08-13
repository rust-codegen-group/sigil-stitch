import type { Config } from './app';
import type { Config as DatabaseConfig } from './database';
import type { Config as ServerConfig } from './server';

export function mergeConfigs( app: Config,
  server: ServerConfig, db: DatabaseConfig ): Config {
  const merged: ServerConfig = { ...app, ...server };
  return { ...merged, ...db } as DatabaseConfig;
}
