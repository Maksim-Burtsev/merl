export namespace Outer {
  export namespace Inner {
    export class Widget {
      spin(turns: number): void {
        console.log(turns);
      }
    }
  }

  export class Gadget {
    spin(turns: number): void {
      console.log(turns);
    }
  }
}

export class Widget {
  spin(turns: number): void {
    console.log(turns);
  }
}
