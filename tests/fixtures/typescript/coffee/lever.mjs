export class Group {
  pressure = 9;

  flush() {
    return this.pressure;
  }

  lock() {
    return this;
  }

  shower() {
    return new Shower();
  }
}

export class Lever extends Group {
  flush() {
    return super.flush() + 1;
  }

  pull(seconds) {
    return seconds;
  }

  release() {
    return this;
  }
}

export class Shower {
  padStart = 2;

  filter(cups) {
    return cups;
  }

  then() {
    return 0;
  }
}

export function makeLever() {
  return new Lever();
}

export function padStart(text) {
  return text;
}
