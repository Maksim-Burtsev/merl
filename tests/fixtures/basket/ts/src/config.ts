export interface Config {
  moduleName?: string;
  //           ^ d: none
  names: Map<string, number>;
  //         ^ d: none
}

export function parse(block: { string: string }): string {
//^ d: none
  return block.string;
}
