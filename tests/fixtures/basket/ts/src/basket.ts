export class Basket {
  items: string[] = [];

  push(item: string): void {
    this.items.push(item);
  }

  includes(item: string): boolean {
    return this.items.includes(item);
  }
}
