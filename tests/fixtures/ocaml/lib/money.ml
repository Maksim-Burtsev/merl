type t = { cents : int }

let format_price { cents } = Printf.sprintf "$%.2f" (float cents /. 100.)
