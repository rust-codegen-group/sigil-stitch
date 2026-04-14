import { Logger } from './logger';
import { formatDate } from './utils';

function greet(name) {
  const date = formatDate();
  Logger.log('Hello, ' + name);
}
