module.exports = class TokenStore {
  getFirstToken(node) {
    return node;
  }

  report(problem) {
    return problem;
  }

  docs = {};

  create(context) {
    return context;
  }

  onCodePathStart() {}
};
