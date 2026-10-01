// Keys of an object literal the file binds to a `const` (#341), on lines of their own and on the
// `const` line itself; not a nested literal's, not a `let` one's, not one behind a call.
const QUOTE_MARKS = {
  double: { quote: '"' },
  backtick: {
    quote: "`",
    alternate: '"',
  },
};
const notice = { messageId: "tick", hints: [] };
let slackKeys = {
  extraKey: 1,
};
const frozen = Object.freeze({
  pinned: 1,
});

export function mark() {
  return [QUOTE_MARKS.backtick, notice.messageId, slackKeys.extraKey, frozen.pinned, QUOTE_MARKS.alternate];
  //                  ^ d: shop/quotes.js:5
  //                                   ^ d: shop/quotes.js:10
  //                                                        ^ d: none
  //                                                                         ^ d: none
  //                                                                                             ^ d: none
}
