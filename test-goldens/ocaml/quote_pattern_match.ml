let describe x = match x with =
  | Some(v) -> Printf.printf "value: %d" v
  | None -> print_endline "none"
