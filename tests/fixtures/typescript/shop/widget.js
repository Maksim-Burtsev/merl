// A script's top-level `var` is a global: `window.ShopWidget` is it (#341).
var ShopWidget = { name: "widget" };

function mountWidget() {
  return window.ShopWidget;
  //            ^ d: shop/widget.js:2
}
