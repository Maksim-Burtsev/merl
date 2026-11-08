const helper = require("./tool");

function outer() {
  run(helper);
  //  ^ d: glean/tool.js:1
  const code = `
  function helper() {}
  `;
  return code;
}

function second(list) {
  if (list) {
    list.forEach(helper);
    //           ^ d: glean/tool.js:1
  }
  /*
  function helper() {}
  */
}

function third() {
  run(() => {
    run(helper);
    //  ^ d: glean/hand.js:27
  });
  function helper() {}
}
