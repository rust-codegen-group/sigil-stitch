import { EventEmitter } from 'events';
import { v4 } from 'uuid';

// extends EventEmitter

/**
 * Application event bus.
 */
export class EventBus extends EventEmitter {
  #handlers;

  constructor() {
    super();
    this.#handlers = new Map();
  }

  publish(event, data) {
    const id = v4();
    this.emit(event, data);
    return id;
  }
}

export function createEventBus() {
  return new EventBus();
}
