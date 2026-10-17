export class Cezve {
  foam = 2;

  get strength() {
    return this.foam * 2;
    //          ^ d: coffee/cezve.js:2
  }

  simmer() {
    const viaArrow = () => this.strength;
    //                          ^ d: coffee/cezve.js:4
    const viaFunction = function () {
      return this.foam;
      //          ^ d: picker coffee/cezve.js:2, coffee/cezve.js:21
    };
    return viaArrow() + viaFunction();
  }
}

export class Moka {
  foam() {
    return 1;
  }
}

export class Ibrik {
  crema = 1;

  crema() {
    return 2;
  }
}

export class Jezve {
  static crema = 1;

  crema() {
    return 3;
  }
}

export class Briki extends Ibrik {}

export class Fs {
  static stat = 1;

  stat() {
    return 4;
  }
}

export class Samovar {
  kind = "m";

  static kind() {
    return "model";
  }

  static tagline() {
    return this.kind();
    //          ^ d: coffee/cezve.js:55
  }
}

export class Dallah {
  decant() {
    return 5;
  }

  decant = 6;
}

export class Lid {
  get = 1;

  get() {
    return 2;
  }
}

export class Kanaka extends Ibrik {
  crema() {
    return super.crema();
    //           ^ d: coffee/cezve.js:29
  }
}

export class Saucer {
  get warmth() { return this._w; }
  set warmth(v) {
    this._w = v;
  }
}
