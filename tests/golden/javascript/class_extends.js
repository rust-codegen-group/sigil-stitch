import { Animal } from './animal';

// Uses Animal

export class Dog extends Animal {
  constructor(name, breed) {
    super(name);
    this.breed = breed;
  }

  speak() {
    return 'Woof!';
  }
}
