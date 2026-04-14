/**
 * A simple counter.
 */
export class Counter {
  count;

  constructor() {
    this.count = 0;
  }

  increment() {
    this.count++;
  }

  getCount() {
    return this.count;
  }
}
