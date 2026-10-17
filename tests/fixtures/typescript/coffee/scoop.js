export function scoop(n) {
  var level = 1;
  for (var i = 0; i < n; i++) {
    var level = 2;
  }
  function inner() {
    var level = 3;
    return level;
    //     ^ d: coffee/scoop.js:7
  }
  [1].forEach(() => {
    var level = 4;
  });
  const t = `
  var level = 5;
  `;
  return level + inner() + t.length;
  //     ^ d: picker coffee/scoop.js:2, coffee/scoop.js:4
}

export function other() {
  var level = 6;
  return level;
  //     ^ d: coffee/scoop.js:22
}
