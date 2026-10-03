let weigh_all xs =
  let acc = ref 0 in
  List.iter (fun x -> acc := !acc + x) xs;
  !acc
(* ^ d: lib/courier.ml:2 *)
