// A script's IIFE keeps its functions out of the global scope, and `window.initMap` publishes
// one: what the IIFE declares is at the top of its script, for other files' `d` too.
(function () {
  function initMap() {}
  window.initMap = initMap;
})();
