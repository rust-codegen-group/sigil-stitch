import type { Logger } from './logging';
import type { User } from './models';
import type { UserRepository } from './repos';

const repo: UserRepository = getRepo();
const logger: Logger = getLogger();
const user: User = repo.findOne();
logger.info('found user');
