import { readFileSync } from 'fs';
import { join } from 'path';

const data = readFileSync('input.txt');
const full = join('dir', 'file');
