export class Block {
  get string(): string {
    return "";
  }

  export(): string {
    return this.string;
  }

  import(path: string): string {
    return path;
  }
}
