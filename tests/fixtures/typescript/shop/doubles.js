// The same in JavaScript (#531).
const pallet = 1;
export const halved = (pallets) => pallets.map(pallet => pallet / 2);
//                                                       ^ d: shop/doubles.js:3
//                                                       status: pallet: local
export const weighed = (pallets) => pallets.map((pallet) => 0);
//                                               ^ d: picker shop/doubles.js:2
//                                               status: pallet: at a declaration, 1 other by name
