"use strict";

module.exports = {
  create(context) {
    function warnAbout(node) {
      context.report({ node });
    }

    return {
      Program(node) {
        return warnAbout(node);
        //     ^ d: scope/a.js:5
      },
    };
  },
};
