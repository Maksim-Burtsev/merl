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
