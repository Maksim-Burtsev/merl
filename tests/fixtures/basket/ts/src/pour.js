export function pour(load, id) {
  let x = "";
  x = load(id);
  return x.brew();
  //       ^ d: ts/src/item.js:2
}
