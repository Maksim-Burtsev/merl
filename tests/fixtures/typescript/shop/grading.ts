// Members behind a qualifier an import names (#341).
import { api, Grade, Hamper, Pace, Ripeness, Stride, Tempo } from "./grades";

export const graded = Grade.Bruised;
//                          ^ d: shop/grades.ts:5
export const ripe = Ripeness.YELLOW;
//                           ^ d: shop/grades.ts:10
export const colors = Hamper.hamperColors;
//                           ^ d: shop/grades.ts:14
// An array's element declares nothing, however an enum member's line it looks.
export const small = Hamper.SMALL;
//                          ^ d: none
// A key directly inside a `const` literal; not in a `let` one, nor behind a call.
export const brisk = Pace.Brisk;
//                        ^ d: shop/grades.ts:22
export const lazy = Tempo.Lazy;
//                        ^ d: none
export const long = Stride.Long;
//                         ^ d: none
// A member of a JavaScript global is none of the project's (#341).
export const parsed = JSON.parse("{}");
//                         ^ d: none
// The project's own member of a global, declared in a `declare global`, is (#341).
export const layer = window.dataLayer;
//                          ^ d: shop/globals.d.ts:6
export const app = globalThis.appConfig;
//                            ^ d: shop/globals.d.ts:8
// A method of an imported `const x = call({…})` is found through the import, as on master.
export const got = api.get("x");
//                     ^ d: shop/grades.ts:43
//                     status: via import
// A member of an instance a module exports as its default (#341).
import settings from "./settings";
import ledgerConf from "./ledger_settings";
export const shopName = settings.SHOP_NAME;
//                               ^ d: shop/settings.ts:3
//                               status: SHOP_NAME → ShopSettings.SHOP_NAME (via settings: ShopSettings)
export const ledgerName = ledgerConf.LEDGER_NAME;
//                                   ^ d: shop/ledger_settings.ts:3
