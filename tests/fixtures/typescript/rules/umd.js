// A UMD bundle hands its module to a factory, the second argument of the wrapper it calls.
(function (global, factory) {
  typeof exports === "object" ? factory(exports) : factory((global.lib = {}));
})(this, function (exports) {
  "use strict";

  function umdHelper() {}
  exports.umdHelper = umdHelper;
});
