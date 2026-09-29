// Packages that are not installed (#392): no `node_modules`, no workspace package of the name.
import { observer } from "mobx-react";
import {
  Leaflet,
  stamp as postmark,
} from "@post/labels";

export const Dispatch = observer(() => postmark(new Leaflet()));
//                      ^ d: shop/dispatch.ts:2
//                      status: observer: via import mobx-react (not installed)
//                                     ^ d: shop/dispatch.ts:5
//                                                  ^ d: shop/dispatch.ts:4
//                                                  status: via import @post/labels (not installed)
