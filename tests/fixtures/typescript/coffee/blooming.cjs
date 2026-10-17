/*
exports.bloom = function () {};
*/
const note = `
exports.bloom = function () {};
`;
module.exports.bloom = bloom;
function bloom() {
  return note;
}
exports.bloom();
//      ^ d: coffee/blooming.cjs:8
