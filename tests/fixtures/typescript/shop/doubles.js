// The same in JavaScript (#531).
const pallet = 1;
export const halved = (pallets) => pallets.map(pallet => pallet / 2);
//                                                       ^ d: shop/doubles.js:3
//                                                       status: pallet: local
