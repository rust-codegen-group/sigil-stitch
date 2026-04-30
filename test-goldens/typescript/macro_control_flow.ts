import type { NotFoundError } from './errors';

if (!user) {
  throw new NotFoundError('not found');
} else {
  return user;
}
